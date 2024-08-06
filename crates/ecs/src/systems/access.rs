use crate::{ComponentMeta, ResourceId};

#[derive(Debug, Default)]
pub struct SystemAccess {
    components: Vec<ComponentAccess>,
    resources: Vec<ResourceAccess>,
    filters: Vec<ComponentAccessFilter>,
    world: Vec<WorldAccess>,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum WorldAccess {
    Immutable,
    Mutable,
    #[default]
    None,
}

impl SystemAccess {
    pub fn world(mut self) -> Self {
        self.world.push(WorldAccess::Immutable);
        self
    }

    pub fn world_mut(mut self) -> Self {
        self.world.push(WorldAccess::Mutable);
        self
    }

    pub fn with(mut self, mut other: SystemAccess) -> Self {
        self.components.append(&mut other.components);
        self.resources.append(&mut other.resources);
        self.filters.append(&mut other.filters);
        self.world.append(&mut other.world);

        self
    }

    pub fn with_component(mut self, param: ComponentAccess) -> Self {
        self.components.push(param);
        self
    }

    pub fn with_resource(mut self, res: ResourceAccess) -> Self {
        self.resources.push(res);
        self
    }

    pub fn with_filter(mut self, filter: ComponentAccessFilter) -> Self {
        self.filters.push(filter);
        self
    }

    // TODO: cannot determine if a system with disjoint mutable and immutable access is valid
    pub fn validate_or_panic(&self) {
        if self.world.len() > 1 {
            panic!("Cannot access mutliple references to the world at once in a system");
        }

        if let Some(world) = self.world.first() {
            match world {
                WorldAccess::None => {}
                WorldAccess::Mutable => {
                    if !self.components.is_empty() || !self.resources.is_empty() {
                        panic!("Cannot mutably access World with any other system paramaters");
                    }
                }
                WorldAccess::Immutable => {
                    if self.is_read_and_write() {
                        panic!("Cannot mutably access Components or Resources while immutably accessing World");
                    }
                }
            }
        }

        let mutable_access: Vec<_> = self.components.iter().filter(|c| c.is_mutable()).collect();
        let immutable_access: Vec<_> = self
            .components
            .iter()
            .filter(|c| c.is_immutable())
            .collect();

        for m in mutable_access.iter() {
            for i in immutable_access.iter() {
                if i.meta.id == m.meta.id {
                    util::tracing::warn!(
                        "Query attemps to access the same Component mutably and immutably: {:#?}, {:#?}",
                        i, m
                    );
                }
            }
        }

        let mutable_access: Vec<_> = self.resources.iter().filter(|c| c.is_mutable()).collect();
        let immutable_access: Vec<_> = self.resources.iter().filter(|c| c.is_immutable()).collect();

        for m in mutable_access.iter() {
            for i in immutable_access.iter() {
                if i.id == m.id {
                    util::tracing::warn!(
                        "System attemps to access the same Resource mutably and immutably: {:#?}, {:#?}",
                        i, m
                    );
                }
            }
        }
    }

    pub fn is_valid(&self) -> bool {
        let mutable_access: Vec<_> = self.components.iter().filter(|c| c.is_mutable()).collect();
        let immutable_access: Vec<_> = self
            .components
            .iter()
            .filter(|c| c.is_immutable())
            .collect();

        for m in mutable_access.iter() {
            for i in immutable_access.iter() {
                if i.meta.id == m.meta.id {
                    util::tracing::error!(
                        "Query attemps to access the same Component mutably and immutably: {:#?}, {:#?}",
                        i, m
                    );
                    return false;
                }
            }
        }

        let mutable_access: Vec<_> = self.resources.iter().filter(|c| c.is_mutable()).collect();
        let immutable_access: Vec<_> = self.resources.iter().filter(|c| c.is_immutable()).collect();

        for m in mutable_access.iter() {
            for i in immutable_access.iter() {
                if i.id == m.id {
                    util::tracing::error!(
                        "System attemps to access the same Resource mutably and immutably: {:#?}, {:#?}",
                        i, m
                    );
                    return false;
                }
            }
        }

        true
    }

    pub fn is_read_only(&self) -> bool {
        !self.components.iter().any(|c| c.is_mutable())
            && !self.resources.iter().any(|r| r.is_mutable())
    }

    pub fn is_read_and_write(&self) -> bool {
        !self.is_read_only()
    }

    pub fn conflicts_with(&self, other: &SystemAccess) -> bool {
        let mutable_access: Vec<_> = self.components.iter().filter(|a| a.is_mutable()).collect();
        let immutable_access: Vec<_> = self
            .components
            .iter()
            .filter(|a| a.is_immutable())
            .collect();

        let other_mutable_access: Vec<_> =
            other.components.iter().filter(|a| a.is_mutable()).collect();
        let other_immutable_access: Vec<_> = other
            .components
            .iter()
            .filter(|a| a.is_immutable())
            .collect();

        let components = mutable_access.iter().any(|s| {
            other_immutable_access
                .iter()
                .any(|o| s.meta.id == o.meta.id)
        }) || other_mutable_access
            .iter()
            .any(|o| immutable_access.iter().any(|s| s.meta.id == o.meta.id))
            || other_mutable_access
                .iter()
                .any(|o| mutable_access.iter().any(|s| s.meta.id == o.meta.id));

        let mutable_access: Vec<_> = self.resources.iter().filter(|a| a.is_mutable()).collect();
        let immutable_access: Vec<_> = self.resources.iter().filter(|a| a.is_immutable()).collect();

        let other_mutable_access: Vec<_> =
            other.resources.iter().filter(|a| a.is_mutable()).collect();
        let other_immutable_access: Vec<_> = other
            .resources
            .iter()
            .filter(|a| a.is_immutable())
            .collect();

        let resources = mutable_access
            .iter()
            .any(|s| other_immutable_access.iter().any(|o| s.id == o.id))
            || other_mutable_access
                .iter()
                .any(|o| immutable_access.iter().any(|s| s.id == o.id))
            || other_mutable_access
                .iter()
                .any(|o| mutable_access.iter().any(|s| s.id == o.id));

        components || resources
    }
}

#[derive(Debug)]
pub struct ComponentAccess {
    pub access_type: AccessType,
    pub meta: ComponentMeta,
}

#[derive(Debug)]
pub struct ResourceAccess {
    pub access_type: AccessType,
    // TODO: resource meta
    pub id: ResourceId,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AccessType {
    Immutable,
    Mutable,
}

impl ComponentAccess {
    pub fn new(access_type: AccessType, meta: ComponentMeta) -> Self {
        Self { access_type, meta }
    }

    pub fn is_immutable(&self) -> bool {
        self.access_type == AccessType::Immutable
    }

    pub fn is_mutable(&self) -> bool {
        self.access_type == AccessType::Mutable
    }
}

impl ResourceAccess {
    pub fn new(access_type: AccessType, id: ResourceId) -> Self {
        Self { access_type, id }
    }

    pub fn is_immutable(&self) -> bool {
        self.access_type == AccessType::Immutable
    }

    pub fn is_mutable(&self) -> bool {
        self.access_type == AccessType::Mutable
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AccessFilter {
    With,
    Without,
    Or,
}

#[derive(Debug)]
pub struct ComponentAccessFilter {
    pub filter: AccessFilter,
    pub meta: ComponentMeta,
}

impl ComponentAccessFilter {
    pub fn new(filter: AccessFilter, meta: ComponentMeta) -> Self {
        Self { filter, meta }
    }

    pub fn with(&self) -> bool {
        self.filter == AccessFilter::With
    }

    pub fn without(&self) -> bool {
        self.filter == AccessFilter::Without
    }

    pub fn or(&self) -> bool {
        self.filter == AccessFilter::Or
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn test() {
        // let sa_1 = SystemAccess::default().with_component(ComponentAccess::new(
        //     AccessType::Immutable,
        //     ComponentId::new(0),
        // ));
        // sa_1.validate_or_panic();

        // let sa_2 = SystemAccess::default().with_component(ComponentAccess::new(
        //     AccessType::Mutable,
        //     ComponentId::new(0),
        // ));
        // sa_2.validate_or_panic();

        // assert!(sa_1.conflicts_with(&sa_2));

        // let panic = SystemAccess::default()
        //     .with_component(ComponentAccess::new(
        //         AccessType::Immutable,
        //         ComponentId::new(0),
        //     ))
        //     .with_component(ComponentAccess::new(
        //         AccessType::Mutable,
        //         ComponentId::new(0),
        //     ));

        // let panic = std::thread::spawn(move || {
        //     panic.validate_or_panic();
        // });

        // assert!(panic.join().is_err());

        // let sa_1 = SystemAccess::default().with_component(ComponentAccess::new(
        //     AccessType::Immutable,
        //     ComponentId::new(0),
        // ));
        // sa_1.validate_or_panic();

        // let sa_2 = SystemAccess::default().with_component(ComponentAccess::new(
        //     AccessType::Mutable,
        //     ComponentId::new(1),
        // ));
        // sa_2.validate_or_panic();

        // assert!(!sa_1.conflicts_with(&sa_2));
    }
}
