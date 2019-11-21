// External crates

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate vulkano;
#[macro_use] extern crate lumberjack;
#[macro_use] extern crate imgui;

extern crate cgmath;
extern crate fnv;
extern crate half;
extern crate hashbrown;
extern crate image;
extern crate noise;
extern crate parking_lot;
extern crate rand;
extern crate rusttype;
extern crate vulkano_shaders;
extern crate winit;
extern crate xalloc;

// modules

pub mod buffer;
pub mod camera;
pub mod compute;
pub mod cpu_pool;
pub mod geometry;
pub mod memory;
pub mod pipeline;
pub mod registry;
pub mod renderer;
pub mod renderpass;
pub mod shader;
pub mod vulkano_win;


#[allow(non_upper_case_globals)]
pub mod lumberjack_scopes {
    use lumberjack::prelude::*;
    use lumberjack::Verbosity::*;

    lazy_static! {
        pub static ref Test:     u32 = lumberjack::register_scope(Scope::new("Test").log(Verbose).print(Warning));
        pub static ref Game:     u32 = lumberjack::register_scope(Scope::new("Game"));
        pub static ref Network:  u32 = lumberjack::register_scope(Scope::new("Network"));
        pub static ref Renderer: u32 = lumberjack::register_scope(Scope::new("Renderer"));
        pub static ref Mesher:   u32 = lumberjack::register_scope(Scope::new("Mesher"));
    }
}