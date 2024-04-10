use super::*;

// macro_rules! queries {
//       ($($t:ident)*) => {
//          #[allow(non_snake_case)]
//           impl<'b, $($t: TypeGetter + Component + Storage),*, Fil: Filter> WorldQuery for Query<'b, ($($t,)*), Fil> {
//             type Output = ($(Ref<'b, $t>,)*);
//
//             fn iter(&self) -> impl Iterator<Item = Self::Output> {
//              // self.world.archetypes
//              //    .iter()
//              //    .filter(|arch| $(arch.contains::<$t>())&&*)
//              //    .filter(|arch| Fil::condition(arch))
//              //    .map(|arch| {
//              //        izip!(
//              //            $(
//              //                {
//              //                    map_vec::<$t>(arch, &self.world.tables[arch.table_id])
//              //                },
//              //            )*
//              //        )
//
//              //    })
//              //    .flatten()
//             }
//
//             // fn get(&self, id: Entity) -> Result<Self::Output, ()> {
//             //     let meta = self.world.get_entity(id).ok_or(())?;
//             //     let len = self.world.archetypes[meta.location.archetype_id].entities.len();
//
//             //     let id_set = vec![$($t::type_id(),)*];
//             //     if !self.world.archetypes[meta.location.archetype_id].contains_id_set(&id_set) {
//             //         return Err(());
//             //     }
//
//             //     (0..len)
//             //         .map(|_| izip!($({
//             //                     map_vec::<$t>(&self.world.archetypes[meta.location.archetype_id], &self.world.tables[meta.location.table_id])
//             //                 },)*)
//             //         )
//             //         .flatten()
//             //         .nth(meta.location.table_row.0)
//             //         .ok_or(())
//             // }
//
//
//             // fn get_single(&self) -> Result<Self::Output, ()> {
//             //     self.world
//             //         .archetypes
//             //         .iter()
//             //         .filter(|arch| $(arch.contains::<$t>())&&*)
//             //         .filter(|arch| Fil::condition(arch))
//             //         .map(|arch| {
//             //             izip!(
//             //                 $(
//             //                     map_vec::<$t>(arch, &self.world.tables[arch.table_id]),
//             //                     )*
//             //                 )
//             //         })
//             //         .flatten()
//             //         .exactly_one()
//             //         .map_err(|_| ())
//             // }
//           }
//
//           impl<'b, $($t: TypeGetter + Component + Storage),*, Fil: Filter> WorldQueryMut for Query<'b, ($($t,)*), Fil> {
//               type Output = ($(RefMut<'b, $t>,)*);
//
//             // fn iter_mut(&mut self) -> impl Iterator<Item = Self::Output> {
//             //  self.world.archetypes
//             //     .iter()
//             //     .filter(|arch| $(arch.contains::<$t>())&&*)
//             //     .filter(|arch| Fil::condition(arch))
//             //     .map(|arch| {
//             //         izip!(
//             //             $(
//             //                 map_vec_mut::<$t>(arch, &self.world.tables[arch.table_id]),
//             //             )*
//             //         )
//
//             //     })
//             //     .flatten()
//             // }
//
//
//             // fn get_mut(&self, id: Entity) -> Result<Self::Output, ()> {
//             //     let meta = self.world.get_entity(id).ok_or(())?;
//             //     let len = self.world.tables[meta.location.table_id].len;
//
//             //     let id_set = vec![$($t::type_id(),)*];
//             //     if !self.world.archetypes[meta.location.archetype_id].contains_id_set(&id_set) {
//             //         return Err(());
//             //     }
//
//             //     (0..len)
//             //         .map(|_| izip!($(
//             //                     map_vec_mut::<$t>(&self.world.archetypes[meta.location.archetype_id], &self.world.tables[meta.location.table_id])
//             //                     ,)*)
//             //         )
//             //         .flatten()
//             //         .nth(meta.location.archetype_index)
//             //         .ok_or(())
//             // }
//
//             // fn get_single_mut(&self) -> Result<Self::Output, ()> {
//             //     self.world
//             //         .archetypes
//             //         .iter()
//             //         .filter(|arch| $(arch.contains::<$t>())&&*)
//             //         .filter(|arch| Fil::condition(arch))
//             //         .map(|arch| {
//             //                 izip!(
//             //                     $(
//             //                         {
//             //                             map_vec_mut::<$t>(arch, &self.world.tables[arch.table_id])
//             //                         },
//             //                         )*
//             //                     )
//             //         })
//             //         .flatten()
//             //         .exactly_one()
//             //         .map_err(|_| ())
//             // }
//           }
//
//          #[allow(non_snake_case)]
//           impl<'b, $($t: TypeGetter + Component + Storage),*, Fil: Filter> WorldQuery for EntityQuery<'b, ($($t,)*), Fil> {
//             type Output = (Entity, $(Ref<'b, $t>,)*);
//
//             fn iter(&self) -> impl Iterator<Item = Self::Output> {
//              self.world.archetypes
//                 .iter()
//                 .filter(|arch| $(arch.contains::<$t>())&&*)
//                 .filter(|arch| Fil::condition(arch))
//                 .map(|arch| {
//                     izip!(
//                         arch.entities.iter().map(|(e, _)| e.clone()),
//                         $(
//                             {
//                                 map_vec::<$t>(arch, &self.world.tables[arch.table_id])
//                             },
//                         )*
//                     )
//
//                 })
//                 .flatten()
//             }
//
//             // fn get(&self, id: Entity) -> Result<Self::Output, ()> {
//             //     let meta = self.world.get_entity(id).ok_or(())?;
//             //     let len = self.world.archetypes[meta.location.archetype_id].entities.len();
//
//             //     let id_set = vec![$($t::type_id(),)*];
//             //     if !self.world.archetypes[meta.location.archetype_id].contains_id_set(&id_set) {
//             //         return Err(());
//             //     }
//
//             //     (0..len)
//             //         .map(|_| izip!(
//             //                 self.world.archetypes[meta.location.archetype_id].entities.iter().map(|(e, _)| e.clone()),
//             //                 $({
//             //                     map_vec::<$t>(&self.world.archetypes[meta.location.archetype_id], &self.world.tables[meta.location.table_id])
//             //                 },)*)
//             //         )
//             //         .flatten()
//             //         .nth(meta.location.table_row.0)
//             //         .ok_or(())
//             // }
//
//             // fn get_single(&self) -> Result<Self::Output, ()> {
//             //     self.world
//             //         .archetypes
//             //         .iter()
//             //         .filter(|arch| $(arch.contains::<$t>())&&*)
//             //         .filter(|arch| Fil::condition(arch))
//             //         .map(|arch| {
//             //             izip!(
//             //                 arch.entities.iter().map(|(e, _)| e.clone()),
//             //                 $(
//             //                     {
//             //                         map_vec::<$t>(arch, &self.world.tables[arch.table_id])
//             //                     },
//             //                     )*
//             //                 )
//             //         })
//             //         .flatten()
//             //         .exactly_one()
//             //         .map_err(|_| ())
//             // }
//           }
//
//          #[allow(non_snake_case)]
//           impl<'b, $($t: TypeGetter + Component + Storage),*, Fil: Filter> WorldQueryMut for EntityQuery<'b, ($($t,)*), Fil> {
//             type Output = (Entity, $(RefMut<'b, $t>,)*);
//
//             // fn iter_mut(&mut self) -> impl Iterator<Item = Self::Output> {
//             //  self.world.archetypes
//             //     .iter()
//             //     .filter(|arch| $(arch.contains::<$t>())&&*)
//             //     .filter(|arch| Fil::condition(arch))
//             //     .map(|arch| {
//             //         izip!(
//             //             arch.entities.iter().map(|(e, _)| e.clone()),
//             //             $(
//             //                 {
//             //                     map_vec_mut::<$t>(arch, &self.world.tables[arch.table_id])
//             //                 },
//             //             )*
//             //         )
//
//             //     })
//             //     .flatten()
//             // }
//
//             // fn get_mut(&self, id: Entity) -> Result<Self::Output, ()> {
//             //     let meta = self.world.get_entity(id).ok_or(())?;
//             //     let len = self.world.tables[meta.location.table_id].len;
//
//             //     let id_set = vec![$($t::type_id(),)*];
//             //     if !self.world.archetypes[meta.location.archetype_id].contains_id_set(&id_set) {
//             //         return Err(());
//             //     }
//
//             //     (0..len)
//             //         .map(|_| izip!(
//             //                 self.world.archetypes[meta.location.archetype_id].entities.iter().map(|(e, _)| e.clone()),
//             //                 $({
//             //                     map_vec_mut::<$t>(&self.world.archetypes[meta.location.archetype_id], &self.world.tables[meta.location.table_id])
//             //                 },)*)
//             //         )
//             //         .flatten()
//             //         .nth(meta.location.archetype_index)
//             //         .ok_or(())
//             // }
//
//             // fn get_single_mut(&self) -> Result<Self::Output, ()> {
//             //     self.world
//             //         .archetypes
//             //         .iter()
//             //         .filter(|arch| $(arch.contains::<$t>())&&*)
//             //         .filter(|arch| Fil::condition(arch))
//             //         .map(|arch| {
//             //             izip!(
//             //                 arch.entities.iter().map(|(e, _)| e.clone()),
//             //                 $(
//             //                     {
//             //                         map_vec_mut::<$t>(arch, &self.world.tables[arch.table_id])
//             //                     },
//             //                     )*
//             //                 )
//             //         })
//             //         .flatten()
//             //         .exactly_one()
//             //         .map_err(|_| ())
//             // }
//           }
//       };
//
//       ($(($t:ident)),*, $next:ident) => {
//           queries!($(($t)),*);
//           queries!($(($t)),*, $next);
//       }
//  }
//
// queries!(A B);
// queries!(A B C);
// queries!(A B C D);
// queries!(A B C D E);
// queries!(A B C D E F);
// queries!(A B C D E F G);
// queries!(A B C D E F G H);
// queries!(A B C D E F G H I);
// queries!(A B C D E F G H I J);
// queries!(A B C D E F G H I J K);
// queries!(A B C D E F G H I J K L);
