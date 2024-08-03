// render_pass.set_pipeline(&sprite_renderer.pipeline);
// // sorted by bind group handle
// render_pass.set_vertex_buffer(0, sprite_renderer.vertex_buffer.slice(..));
// // sorted by bind group handle
// render_pass.set_vertex_buffer(1, sprite_renderer.sprite_buffer.slice(..));
// // sorted by bind group handle
// render_pass.set_vertex_buffer(2, sprite_renderer.transform_buffer.slice(..));
// // sorted by bind group handle
// render_pass.set_bind_group(0, &sprite_renderer.atlas_uniform_bind_group, &[]);
//
// let mut offset = 0;
// let previous_bind_index = usize::MAX;
// for (_, _, handle, _, anim) in sprites.iter() {
//     if (**handle).index() != previous_bind_index {
//         let binding = if anim.is_some() {
//             atlas_bind_groups.get(**handle).unwrap()
//         } else {
//             bind_groups.get(**handle).unwrap()
//         };
//
//         render_pass.set_bind_group(1, binding, &[]);
//     }
//
//     render_pass.draw(
//         offset * VERTICES..offset * VERTICES + VERTICES,
//         offset..offset + 1,
//     );
//     offset += 1;
// }
