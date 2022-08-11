use std::ops::Deref;
use std::sync::Arc;
use egui::{ClippedPrimitive, ImageData, Shape, TextureId, TexturesDelta};
use egui::epaint::ahash::AHashMap;
use egui::epaint::Primitive;
use skia_safe::{Bitmap, BlendMode, Canvas, ClipOp, Color, ConditionallySend, Data, Drawable, Image, ImageInfo, IRect, Paint, PictureRecorder, Point, Rect, scalar, Sendable, Size, Surface, Vertices};
use skia_safe::image::BitDepth;
use skia_safe::vertices::{Builder, BuilderFlags, VertexMode};
use skia_safe::wrapper::NativeTransmutableWrapper;

struct PaintHandle {
    paint: Paint,
    image: Image,
}

pub struct Painter {
    paints: AHashMap<TextureId, PaintHandle>,
}

impl Painter {
    pub fn new() -> Painter {
        Self {
            paints: AHashMap::new(),
        }
    }

    pub fn paint_and_update_textures(
        &mut self,
        surface: &mut Surface,
        dpi: f32,
        primitives: Vec<ClippedPrimitive>,
        textures_delta: TexturesDelta,
    ) {
        textures_delta.set.iter().for_each(|(id, image)| {
            let delta_image = match &image.image {
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

            let image = match image.pos {
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
            let sampling_options = skia_safe::SamplingOptions::new(
                skia_safe::FilterMode::Nearest,
                skia_safe::MipmapMode::None,
            );
            let tile_mode = skia_safe::TileMode::Repeat;

            let font_shader = image
                .to_shader((tile_mode, tile_mode), sampling_options, &local_matrix)
                .unwrap();

            image.width();

            let mut paint = Paint::default();
            paint.set_shader(font_shader);
            paint.set_color(Color::WHITE);

            self.paints.insert(id.clone(), PaintHandle { paint, image });
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

                    for mut mesh in mesh.split_to_u16() {
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

                        let mut vertex_builder = Builder::new(
                            VertexMode::Triangles,
                            mesh.vertices.len(),
                            mesh.indices.len(),
                            BuilderFlags::HAS_COLORS | BuilderFlags::HAS_TEX_COORDS,
                        );

                        {
                            let indices = vertex_builder.indices().expect("indices");
                            indices.copy_from_slice(mesh.indices.as_slice());
                        }

                        for (i, v) in mesh.vertices.iter().enumerate() {
                            vertex_builder.positions()[i] = Point::new(v.pos.x, v.pos.y);
                            vertex_builder.tex_coords().unwrap()[i] = Point::new(v.uv.x, v.uv.y);
                            vertex_builder.colors().unwrap()[i] = Color::from_argb(
                                v.color.a(),
                                v.color.r(),
                                v.color.g(),
                                v.color.b(),
                            );
                        }

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
                        arc.draw_vertices(
                            &vertices,
                            BlendMode::Modulate,
                            &self.paints[&texture_id].paint,
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
