pub trait Plugin {
    fn build(&mut self, app: &mut crate::app::App);
}

pub trait PluginSet {
    fn get(self) -> Vec<Box<dyn Plugin>>;
}

pub trait IntoPlugin {
    type Plugin: Plugin;

    fn into_plugin(self) -> Box<Self::Plugin>;
}

impl<T: Plugin> IntoPlugin for T {
    type Plugin = T;

    fn into_plugin(self) -> Box<Self::Plugin> {
        Box::new(self)
    }
}

impl<P> PluginSet for P
where
    P: Plugin + 'static,
{
    fn get(self) -> Vec<Box<dyn Plugin>> {
        vec![self.into_plugin()]
    }
}

macro_rules! impl_plugin_set {
    ($($t:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($t: Plugin + 'static),*> PluginSet for ($($t,)*)
                {
                    fn get(self) -> Vec<Box<dyn Plugin>> {
                        let ($($t,)*) = self;

                        vec![
                            $($t.into_plugin(),)*
                        ]
                    }
                }
    }
}

ecs::ecs_derive::all_tuples!(impl_plugin_set, 2, 10, F);
