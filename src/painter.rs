use std::ops::Deref;
use std::sync::Arc;

use egui::epaint::ahash::AHashMap;
use egui::epaint::{Mesh16, Primitive};
use egui::{ClippedPrimitive, ImageData, Pos2, TextureFilter, TextureId, TexturesDelta};
use skia_safe::vertices::VertexMode;
use skia_safe::wrapper::ValueWrapper;
use skia_safe::{
    scalar, BlendMode, Canvas, ClipOp, Color, ConditionallySend, Data, Drawable, Image, ImageInfo,
    Paint, PictureRecorder, Point, Rect, Sendable, Surface, Vertices,
};

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
        canvas: &mut Canvas,
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
                    let pixels = font.srgba_pixels(Some(1.0));
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

            #[cfg(feature = "cpu_fix")]
            let sampling_options = skia_safe::SamplingOptions::new(
                skia_safe::FilterMode::Nearest,
                skia_safe::MipmapMode::None,
            );
            #[cfg(not(feature = "cpu_fix"))]
            let sampling_options = {
                let filter_mode = match image_delta.options.magnification {
                    TextureFilter::Nearest => skia_safe::FilterMode::Nearest,
                    TextureFilter::Linear => skia_safe::FilterMode::Linear,
                };
                let mm_mode = match image_delta.options.minification {
                    TextureFilter::Nearest => skia_safe::MipmapMode::Nearest,
                    TextureFilter::Linear => skia_safe::MipmapMode::Linear,
                };
                let sampling_options = skia_safe::SamplingOptions::new(filter_mode, mm_mode);
                sampling_options
            };
            let tile_mode = skia_safe::TileMode::Clamp;

            let font_shader = image
                .to_shader((tile_mode, tile_mode), sampling_options, &local_matrix)
                .unwrap();

            image.width();

            let mut paint = Paint::default();
            paint.set_shader(font_shader);
            paint.set_color(Color::WHITE);

            self.paints.insert(
                id.clone(),
                PaintHandle {
                    paint,
                    image,
                    paint_type: match image_delta.image {
                        ImageData::Color(_) => PaintType::Image,
                        ImageData::Font(_) => PaintType::Font,
                    },
                },
            );
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
                    canvas.set_matrix(&skia_safe::M44::new_identity().set_scale(dpi, dpi, 1.0));
                    let mut arc = skia_safe::AutoCanvasRestore::guard(canvas, true);

                    #[cfg(feature = "cpu_fix")]
                    let meshes = mesh
                        .split_to_u16()
                        .into_iter()
                        .flat_map(|mesh| self.split_texture_meshes(mesh))
                        .collect::<Vec<Mesh16>>();
                    #[cfg(not(feature = "cpu_fix"))]
                    let meshes = mesh.split_to_u16();

                    for mesh in &meshes {
                        let texture_id = mesh.texture_id;

                        let mut pos = Vec::with_capacity(mesh.vertices.len());
                        let mut texs = Vec::with_capacity(mesh.vertices.len());
                        let mut colors = Vec::with_capacity(mesh.vertices.len());

                        mesh.vertices.iter().enumerate().for_each(|(_i, v)| {
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

                        // Egui use the uv coordinates 0,0 to get a white color when drawing vector graphics
                        // 0,0 is always a white dot on the font texture
                        // Unfortunately skia has a bug where it cannot get a color when the uv coordinates are equal
                        // https://bugs.chromium.org/p/skia/issues/detail?id=13706
                        // As a workaround, split_texture_meshes splits meshes that contain both 0,0 vertices, as
                        // well as non-0,0 vertices into multiple meshes.
                        // Here we check if the mesh is a font texture and if it's first uv has 0,0
                        // If yes, we use a white paint instead of the texture shader paint

                        let cpu_fix = if cfg!(feature = "cpu_fix")
                            && self.paints.get(&mesh.texture_id).unwrap().paint_type
                                == PaintType::Font
                        {
                            !texs
                                .first()
                                .map(|point| point.x != 0.0 || point.y != 0.0)
                                .unwrap()
                        } else {
                            false
                        };

                        let paint = if cpu_fix {
                            &self.white_paint_workaround
                        } else {
                            &self.paints[&texture_id].paint
                        };

                        arc.draw_vertices(&vertices, BlendMode::Modulate, paint);
                    }
                }
                Primitive::Callback(data) => {
                    let callback: Arc<EguiSkiaPaintCallback> = data.callback.downcast().unwrap();
                    let rect = data.rect;

                    let skia_rect = Rect::new(
                        rect.min.x * dpi,
                        rect.min.y * dpi,
                        rect.max.x * dpi,
                        rect.max.y * dpi,
                    );

                    let mut drawable: Drawable = callback.callback.deref()(skia_rect).0.unwrap();

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
    fn split_texture_meshes(&self, mesh: Mesh16) -> Vec<Mesh16> {
        if self.paints.get(&mesh.texture_id).unwrap().paint_type != PaintType::Font {
            return vec![mesh];
        }

        let mut is_zero = None;

        let mut meshes = Vec::new();
        meshes.push(Mesh16 {
            indices: vec![],
            vertices: vec![],
            texture_id: mesh.texture_id,
        });

        for index in mesh.indices.iter() {
            let vertex = mesh.vertices.get(*index as usize).unwrap();
            let is_current_zero = (vertex.uv.x == 0.0 && vertex.uv.y == 0.0);
            if is_current_zero != is_zero.unwrap_or(is_current_zero) {
                meshes.push(Mesh16 {
                    indices: vec![],
                    vertices: vec![],
                    texture_id: mesh.texture_id,
                });
                is_zero = Some(is_current_zero)
            }
            if is_zero.is_none() {
                is_zero = Some(is_current_zero)
            }
            let last = meshes.last_mut().unwrap();
            last.vertices.push(vertex.clone());
            last.indices.push(last.indices.len() as u16);
        }

        meshes
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
                SyncSendableDrawable(
                    pr.finish_recording_as_drawable()
                        .unwrap()
                        .wrap_send()
                        .unwrap(),
                )
            }),
        }
    }
}

struct SyncSendableDrawable(pub Sendable<Drawable>);

unsafe impl Sync for SyncSendableDrawable {}
