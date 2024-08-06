use std::fmt::Debug;

use ecs_macro::all_tuples;

use crate::{
    access::SystemAccess, ArchId, Archetype, IntoCondition, IntoSystem, OneShotSystems,
    StoredCondition, StoredSystem, World,
};

pub trait LabelId: 'static + Debug {
    fn id(&self) -> usize;
}

pub trait SystemSetLabel: LabelId {}

#[derive(Debug)]
struct SystemWithConditions {
    system: StoredSystem,
    condition_indexes: Vec<usize>,
    tag: Option<Box<dyn SystemSetLabel>>,
}

impl SystemWithConditions {
    pub fn new<M>(system: impl IntoSystem<M>) -> Self {
        Self {
            system: Box::new(system.into_system()),
            condition_indexes: Vec::new(),
            tag: None,
        }
    }

    pub fn add_condition(&mut self, condition: usize) {
        self.condition_indexes.push(condition);
    }

    pub fn shift_condition_indexes(&mut self, amt: usize) {
        for index in self.condition_indexes.iter_mut() {
            *index += amt;
        }
    }
}

#[derive(Debug, Default)]
pub struct SystemSet {
    systems: Vec<SystemWithConditions>,
    conditions: Vec<StoredCondition>,
    condition: Option<StoredCondition>,
    archetypes_len: usize,
}

impl SystemSet {
    pub fn join_disjoint(sets: Vec<Self>) -> Self {
        let mut new_set = SystemSet::default();

        for mut set in sets.into_iter() {
            if let Some(condition) = set.condition.take() {
                let index = new_set.conditions.len();
                new_set.conditions.push(condition);

                for mut system in set.systems.into_iter() {
                    system.shift_condition_indexes(new_set.conditions.len());
                    system.add_condition(index);
                    new_set.systems.push(system);
                }
            } else {
                for mut system in set.systems.into_iter() {
                    system.shift_condition_indexes(new_set.conditions.len());
                    new_set.systems.push(system);
                }
            }

            for condition in set.conditions.into_iter() {
                new_set.conditions.push(condition);
            }
        }

        new_set
    }

    pub fn access(&self, world: &mut World) -> Vec<SystemAccess> {
        let mut access = self
            .systems
            .iter()
            .map(|s| s.system.access(world))
            .collect::<Vec<_>>();
        access.extend(self.conditions.iter().map(|c| c.access(world)));
        access
    }

    pub fn run_if<M, F: IntoCondition<M>>(mut self, condition: F) -> Self {
        self.condition = Some(Box::new(condition.into_system()));
        self
    }

    pub fn validate_systems_and_conditions_or_panic(&self, world: &mut World) {
        for system in self.access(world).iter() {
            system.validate_or_panic();
        }
    }

    pub fn init_state(&mut self, world: &mut World) {
        for s in self.systems.iter_mut() {
            s.system.init_state(world);
        }

        for c in self.conditions.iter_mut() {
            c.init_state(world);
        }

        if let Some(c) = &mut self.condition {
            c.init_state(world);
        }
    }

    pub fn apply_deffered(&mut self, world: &mut World, one_shot_systems: &mut OneShotSystems) {
        for s in self.systems.iter_mut() {
            s.system.apply_deffered(world, one_shot_systems);
        }

        let mut indexes = Vec::with_capacity(one_shot_systems.len());
        let mut temp = OneShotSystems::default();
        for (index, (one_shot, condition)) in one_shot_systems.iter_mut() {
            if condition.run_unsafe(unsafe { world.as_unsafe_world() }) {
                indexes.push(index);
                one_shot.init_state(world);
                one_shot.run_unsafe(unsafe { world.as_unsafe_world() });
                one_shot.apply_deffered(world, &mut temp);
            }
        }
        one_shot_systems.remove_indexes(indexes.into_iter());
        one_shot_systems.append(temp);
    }

    pub fn new_archetype(&mut self, arch: &Archetype, one_shot_systems: &mut OneShotSystems) {
        for s in self.systems.iter_mut() {
            s.system.new_archetype(arch);
        }
        one_shot_systems.new_archetype(arch);
    }

    pub fn run(&mut self, world: &mut World) {
        if let Some(c) = &mut self.condition {
            if !c.run_unsafe(unsafe { world.as_unsafe_world() }) {
                return;
            }
        }

        let pre_deffered_arch_len = world.archetypes.len();
        for s in self.systems.iter_mut() {
            for arch_id in self.archetypes_len..pre_deffered_arch_len {
                let arch = world
                    .archetypes
                    .get(ArchId::new(arch_id))
                    .expect("valid id");
                s.system.new_archetype(arch);
            }

            let world = unsafe { world.as_unsafe_world() };
            if s.condition_indexes
                .iter()
                .all(|i| self.conditions[*i].run_unsafe(world))
            {
                s.system.run_unsafe(world);
            }
        }
        self.archetypes_len = pre_deffered_arch_len;
    }
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `System`/`SystemSet`",
    label = "invalid `System`",
    note = "only include `Resource` and `Component` types in the system signature"
)]
pub trait IntoSystemStorage<Marker>
where
    Self: Sized,
{
    fn into_set(self) -> SystemSet;
    fn run_if<M>(self, condition: impl IntoCondition<M>) -> SystemSet;
}

impl IntoSystemStorage<()> for SystemSet {
    fn into_set(self) -> SystemSet {
        self
    }
    fn run_if<M>(self, condition: impl IntoCondition<M>) -> SystemSet {
        self.run_if(condition)
    }
}

impl<Marker, F> IntoSystemStorage<Marker> for F
where
    F: IntoSystem<Marker>,
{
    fn into_set(self) -> SystemSet {
        SystemSet {
            systems: vec![SystemWithConditions::new(self)],
            conditions: Vec::new(),
            condition: None,
            archetypes_len: 0,
        }
    }
    fn run_if<M>(self, condition: impl IntoCondition<M>) -> SystemSet {
        self.into_set().run_if(condition)
    }
}

macro_rules! impl_into_system_tuple {
    ($(($t:ident, $p:ident)),*) => {
        #[allow(non_snake_case)]
        impl<$($t, $p),*> IntoSystemStorage<($($p,)*)> for ($($t,)*)
            where
                $($t: IntoSystemStorage<$p>,)*
        {
            fn into_set(self) -> SystemSet {
                let ($($t,)*) = self;

                SystemSet::join_disjoint(vec![$($t.into_set(),)*])
            }
            fn run_if<M>(self, condition: impl IntoCondition<M>) -> SystemSet {
                self.into_set().run_if(condition)
            }
        }
    }
}

all_tuples!(impl_into_system_tuple, 1, 15, F, P);
