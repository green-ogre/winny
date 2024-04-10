use super::*;

// pub struct EntityQuery<'a, T, F = ()> {
//     world: &'a World,
//     query: PhantomData<T>,
//     filter: PhantomData<F>,
// }
//
// impl<'a, T, F> EntityQuery<'a, T, F> {
//     pub fn new(world: &'a World) -> Self {
//         Self {
//             world,
//             query: PhantomData,
//             filter: PhantomData,
//         }
//     }
// }
//
// impl<'b, T: TypeGetter + Component + Storage, F: Filter> WorldQuery for EntityQuery<'b, T, F> {
//     type Output = (Entity, Ref<'b, T>);
//
//     fn iter(&self) -> impl Iterator<Item = Self::Output> {
//         self.world
//             .archetypes
//             .iter()
//             .filter(|arch| arch.contains::<T>())
//             .filter(|arch| F::condition(arch))
//             .map(|arch| {
//                 izip!(
//                     arch.entities.iter().map(|(e, _)| e.clone()),
//                     map_vec::<T>(arch, &self.world.tables[arch.table_id]),
//                 )
//             })
//             .flatten()
//     }
//
//     // fn get(&self, id: Entity) -> Result<Self::Output, ()> {
//     //     let meta = self.world.get_entity(id).ok_or(())?;
//     //     let len = self.world.archetypes[meta.location.archetype_id]
//     //         .entities
//     //         .len();
//
//     //     let id_set = vec![T::type_id()];
//     //     if !self.world.archetypes[meta.location.archetype_id].contains_id_set(&id_set) {
//     //         return Err(());
//     //     }
//
//     //     (0..len)
//     //         .map(|_| {
//     //             izip!(
//     //                 self.world.archetypes[meta.location.archetype_id]
//     //                     .entities
//     //                     .iter()
//     //                     .map(|(e, _)| e.clone()),
//     //                 map_vec::<T>(
//     //                     &self.world.archetypes[meta.location.archetype_id],
//     //                     &self.world.tables[meta.location.table_id],
//     //                 ),
//     //             )
//     //         })
//     //         .flatten()
//     //         .nth(meta.location.table_row.0)
//     //         .ok_or(())
//     // }
//
//     // fn get_single(&self) -> Result<Self::Output, ()> {
//     //     self.world
//     //         .archetypes
//     //         .iter()
//     //         .filter(|arch| arch.contains::<T>())
//     //         .filter(|arch| F::condition(arch))
//     //         .map(|arch| {
//     //             izip!(
//     //                 arch.entities.iter().map(|(e, _)| e.clone()),
//     //                 map_vec::<T>(arch, &self.world.tables[arch.table_id]),
//     //             )
//     //         })
//     //         .flatten()
//     //         .exactly_one()
//     //         .map_err(|_| ())
//     // }
// }
//
// impl<'b, T: TypeGetter + Component + Storage, F: Filter> WorldQueryMut for EntityQuery<'b, T, F> {
//     type Output = (Entity, RefMut<'b, T>);
//
//     // fn iter_mut(&mut self) -> impl Iterator<Item = Self::Output> {
//     //     self.world
//     //         .archetypes
//     //         .iter()
//     //         .filter(|arch| arch.contains::<T>())
//     //         .filter(|arch| F::condition(arch))
//     //         .map(|arch| {
//     //             izip!(
//     //                 arch.entities.iter().map(|(e, _)| e.clone()),
//     //                 map_vec_mut::<T>(arch, &self.world.tables[arch.table_id]),
//     //             )
//     //         })
//     //         .flatten()
//     // }
//
//     // fn get_mut(&self, id: Entity) -> Result<Self::Output, ()> {
//     //     let meta = self.world.get_entity(id).ok_or(())?;
//     //     let len = self.world.archetypes[meta.location.archetype_id]
//     //         .entities
//     //         .len();
//
//     //     let id_set = vec![T::type_id()];
//     //     if !self.world.archetypes[meta.location.archetype_id].contains_id_set(&id_set) {
//     //         return Err(());
//     //     }
//
//     //     (0..len)
//     //         .map(|_| {
//     //             izip!(
//     //                 self.world.archetypes[meta.location.archetype_id]
//     //                     .entities
//     //                     .iter()
//     //                     .map(|(e, _)| e.clone()),
//     //                 map_vec_mut::<T>(
//     //                     &self.world.archetypes[meta.location.archetype_id],
//     //                     &self.world.tables[meta.location.table_id],
//     //                 ),
//     //             )
//     //         })
//     //         .flatten()
//     //         .nth(meta.location.archetype_index)
//     //         .ok_or(())
//     // }
//
//     // fn get_single_mut(&self) -> Result<Self::Output, ()> {
//     //     self.world
//     //         .archetypes
//     //         .iter()
//     //         .filter(|arch| arch.contains::<T>())
//     //         .filter(|arch| F::condition(arch))
//     //         .map(|arch| {
//     //             izip!(
//     //                 arch.entities.iter().map(|(e, _)| e.clone()),
//     //                 map_vec_mut::<T>(arch, &self.world.tables[arch.table_id]),
//     //             )
//     //         })
//     //         .flatten()
//     //         .exactly_one()
//     //         .map_err(|_| ())
//     // }
// }
