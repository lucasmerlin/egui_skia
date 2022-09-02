use std::ops::Deref;
use std::sync::Arc;

use egui::{ClippedPrimitive, ImageData, TextureFilter, TextureId, TexturesDelta};
use egui::epaint::ahash::AHashMap;
use egui::epaint::{Mesh16, Primitive, Vertex};
use skia_safe::{BlendMode, Canvas, ClipOp, Color, ConditionallySend, Data, Drawable, Image, ImageInfo, Paint, PictureRecorder, Point, Rect, scalar, Sendable, Surface, Vertices};
use skia_safe::vertices::{VertexMode};
use skia_safe::wrapper::NativeTransmutableWrapper;

#[derive(Eq, PartialEq)]
enum PaintType {
    Image,
    Font,
}

struct PaintHandle {
    paint: Paint,
    image: Image,
    paint_type: PaintType,
}

pub struct Painter {
    paints: AHashMap<TextureId, PaintHandle>,
    white_paint_workaround: Paint,
}

impl Painter {
    pub fn new() -> Painter {

        let mut white_paint_workaround = Paint::default();
        white_paint_workaround.set_color(Color::WHITE);

        Self {
            paints: AHashMap::new(),
            white_paint_workaround,
        }
    }

    pub fn paint_and_update_textures(
        &mut self,
        surface: &mut Surface,
        dpi: f32,
        primitives: Vec<ClippedPrimitive>,
        textures_delta: TexturesDelta,
    ) {
        textures_delta.set.iter().for_each(|(id, image_delta)| {
            let delta_image = match &image_delta.image {
                ImageData::Color(color_image) => Image::from_raster_data(
                    &ImageInfo::new_n32_premul(
                        skia_safe::ISize::new(
                            color_image.width() as i32,
                            color_image.height() as i32,
                        ),
                        None,
                    ),
                    Data::new_copy(
                        color_image
                            .pixels
                            .iter()
                            .flat_map(|p| p.to_array())
                            .collect::<Vec<_>>()
                            .as_slice(),
                    ),
                    color_image.width() * 4,
                )
                    .unwrap(),
                ImageData::Font(font) => {
                    let pixels = font.srgba_pixels(1.0);
                    Image::from_raster_data(
                        &ImageInfo::new_n32_premul(
                            skia_safe::ISize::new(font.width() as i32, font.height() as i32),
                            None,
                        ),
                        Data::new_copy(
                            pixels
                                .flat_map(|p| p.to_array())
                                .collect::<Vec<_>>()
                                .as_slice(),
                        ),
                        font.width() * 4,
                    )
                        .unwrap()
                }
            };

            let image = match image_delta.pos {
                None => delta_image,
                Some(pos) => {
                    let old_image = self.paints.remove(&id).unwrap().image;

                    let mut surface = Surface::new_raster_n32_premul(skia_safe::ISize::new(
                        old_image.width() as i32,
                        old_image.height() as i32,
                    ))
                        .unwrap();

                    let canvas = surface.canvas();

                    canvas.draw_image(&old_image, Point::new(0.0, 0.0), None);

                    canvas.clip_rect(
                        Rect::new(
                            pos[0] as scalar,
                            pos[1] as scalar,
                            (pos[0] as i32 + delta_image.width()) as scalar,
                            (pos[1] as i32 + delta_image.height()) as scalar,
                        ),
                        ClipOp::default(),
                        false,
                    );

                    canvas.clear(Color::TRANSPARENT);
                    canvas.draw_image(&delta_image, Point::new(pos[0] as f32, pos[1] as f32), None);

                    surface.image_snapshot()
                }
            };

            let local_matrix =
                skia_safe::Matrix::scale((1.0 / image.width() as f32, 1.0 / image.height() as f32));
            let filter_mode = match image_delta.filter {
                TextureFilter::Nearest => { skia_safe::FilterMode::Nearest }
                TextureFilter::Linear => { skia_safe::FilterMode::Linear }
            };
            let sampling_options = skia_safe::SamplingOptions::new(
                filter_mode,
                skia_safe::MipmapMode::None,
            );
            let tile_mode = skia_safe::TileMode::Clamp;

            let font_shader = image
                .to_shader((tile_mode, tile_mode), sampling_options, &local_matrix)
                .unwrap();

            image.width();

            let mut paint = Paint::default();
            paint.set_shader(font_shader);
            paint.set_color(Color::WHITE);

            self.paints.insert(id.clone(), PaintHandle { paint, image, paint_type: match image_delta.image {
                ImageData::Color(_) => PaintType::Image,
                ImageData::Font(_) => PaintType::Font,
            } });
        });

        for primitive in primitives {
            let skclip_rect = Rect::new(
                primitive.clip_rect.min.x,
                primitive.clip_rect.min.y,
                primitive.clip_rect.max.x,
                primitive.clip_rect.max.y,
            );
            match primitive.primitive {
                Primitive::Mesh(mesh) => {
                    let canvas = surface.canvas();
                    canvas.set_matrix(&skia_safe::M44::new_identity().set_scale(dpi, dpi, 1.0));
                    let mut arc = skia_safe::AutoCanvasRestore::guard(canvas, true);

                    // Egui use the uv coordinates 0,0 to get a white color when drawing vector graphics
                    // 0,0 is always a white dot on the font texture
                    // Unfortunately skia has a bug where it cannot get a color when the uv coordinates are equal
                    // https://bugs.chromium.org/p/skia/issues/detail?id=13706
                    // As a workaround, we change the 2nd and 3rd vertex to make a small triangle on the first pixel.

                    let mut meshes = mesh.split_to_u16();
                    #[cfg(feature = "cpu_fix")]
                    meshes.iter_mut().for_each(|mesh| self.fix_zero_vertices(mesh));

                    for mesh in &meshes {
                        let texture_id = mesh.texture_id;

                        let mut pos = Vec::with_capacity(mesh.vertices.len());
                        let mut texs = Vec::with_capacity(mesh.vertices.len());
                        let mut colors = Vec::with_capacity(mesh.vertices.len());

                        mesh.vertices.iter().for_each(|v| {
                            pos.push(Point::new(v.pos.x, v.pos.y));
                            texs.push(Point::new(v.uv.x, v.uv.y));
                            colors.push(Color::from_argb(
                                v.color.a(),
                                v.color.r(),
                                v.color.g(),
                                v.color.b(),
                            ));
                        });

                        // TODO: Use vertex builder
                        // let mut vertex_builder = Builder::new(
                        //     VertexMode::Triangles,
                        //     mesh.vertices.len(),
                        //     mesh.indices.len(),
                        //     BuilderFlags::HAS_COLORS | BuilderFlags::HAS_TEX_COORDS,
                        // );
                        //
                        // for (i, v) in mesh.vertices.iter().enumerate() {
                        //     vertex_builder.positions()[i] = Point::new(v.pos.x, v.pos.y);
                        //     vertex_builder.tex_coords().unwrap()[i] = Point::new(v.uv.x, v.uv.y);
                        //     vertex_builder.colors().unwrap()[i] = Color::from_argb(
                        //         v.color.a(),
                        //         v.color.r(),
                        //         v.color.g(),
                        //         v.color.b(),
                        //     );
                        // }
                        // let vertices = vertex_builder.detach();

                        let vertices = Vertices::new_copy(
                            VertexMode::Triangles,
                            &pos,
                            &texs,
                            &colors,
                            Some(
                                &mesh
                                    .indices
                                    .iter()
                                    .map(|index| *index as u16)
                                    .collect::<Vec<u16>>()
                                    .as_slice(),
                            ),
                        );

                        arc.clip_rect(skclip_rect, ClipOp::default(), true);

                        let paint= &self.paints[&texture_id].paint;

                        arc.draw_vertices(
                            &vertices,
                            BlendMode::Modulate,
                            paint,
                        );
                    }
                }
                Primitive::Callback(data) => {
                    let callback: Arc<EguiSkiaPaintCallback> = data.callback.downcast().unwrap();
                    let rect = data.rect;

                    let skia_rect = Rect::new(
                        rect.min.x,
                        rect.min.y,
                        rect.max.x,
                        rect.max.y,
                    );

                    let mut drawable: Drawable = callback.callback.deref()(skia_rect).0.unwrap();

                    let canvas = surface.canvas();

                    let mut arc = skia_safe::AutoCanvasRestore::guard(canvas, true);

                    arc.clip_rect(skclip_rect, ClipOp::default(), true);
                    arc.translate((rect.min.x, rect.min.y));

                    drawable.draw(&mut arc, None);
                }
            }
        }

        textures_delta.free.iter().for_each(|id| {
            self.paints.remove(id);
        });
    }


    // This could be optimized more but works for now
    #[cfg(feature = "cpu_fix")]
    fn fix_zero_vertices(&self, mesh: &mut Mesh16) {

        let texture = self.paints.get(&mesh.texture_id).unwrap();
        let w = texture.image.width();
        let h = texture.image.height();

        let is_null = |vertex: &Vertex| {
            vertex.uv.x == 0.0 && vertex.uv.y == 0.0
        };

        for a in 0..mesh.indices.len() / 3 {
            let i = a * 3;
            let index = mesh.indices[i];
            let vertex = mesh.vertices.get(index as usize).unwrap();

            if is_null(vertex) {
                let index2 = mesh.indices[i+1];
                let index3 = mesh.indices[i+2];

                let mut vertex2 = mesh.vertices.get(index2 as usize).unwrap().clone();
                let mut vertex3 = mesh.vertices.get(index3 as usize).unwrap().clone();

                if is_null(&vertex2) && is_null(&vertex3) {
                    vertex2.uv.x = 0.5 / w as f32;
                    vertex3.uv.x = 0.5 / w as f32;
                    vertex3.uv.y = 0.5 / h as f32;

                    mesh.indices[i+1] = mesh.vertices.len() as u16;
                    mesh.indices[i+2] = mesh.vertices.len() as u16 + 1;

                    mesh.vertices.push(vertex2);
                    mesh.vertices.push(vertex3);
                }

            }

        }
    }
}


pub struct EguiSkiaPaintCallback {
    callback: Box<dyn Fn(Rect) -> SyncSendableDrawable + Send + Sync>,
}

impl EguiSkiaPaintCallback {
    pub fn new<F: Fn(&mut Canvas) + Send + Sync + 'static>(callback: F) -> EguiSkiaPaintCallback {
        EguiSkiaPaintCallback {
            callback: Box::new(move |rect| {
                let mut pr = PictureRecorder::new();
                let mut canvas = pr.begin_recording(rect, None);
                callback(&mut canvas);
                SyncSendableDrawable(pr.finish_recording_as_drawable().unwrap().wrap_send().unwrap())
            }),
        }
    }
}

struct SyncSendableDrawable(pub Sendable<Drawable>);

unsafe impl Sync for SyncSendableDrawable {}
