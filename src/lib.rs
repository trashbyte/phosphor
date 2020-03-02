// External crates

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate vulkano;
#[macro_use] extern crate hemlock;
extern crate imgui;

// modules

pub mod buffer;
pub mod camera;
pub mod compute;
pub mod cpu_pool;
pub mod geometry;
pub mod memory;
#[macro_use] mod names;
// pub mod pipeline;
pub mod renderer;
pub mod renderpass;
pub mod shader;
pub mod vulkano_win;
pub mod stage;
pub mod material;


#[allow(non_upper_case_globals)]
pub mod hemlock_scopes {
    use hemlock::prelude::*;
    use hemlock::Verbosity::*;

    lazy_static! {
        pub static ref Test:     u32 = hemlock::register_scope(Scope::new("Test").log(Verbose).print(Warning));
        pub static ref Renderer: u32 = hemlock::register_scope(Scope::new("Renderer"));
    }
}
