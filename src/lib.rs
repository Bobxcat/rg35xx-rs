pub mod app;
pub mod rg35xx;
pub mod sim;

mod distance_integrator;
mod menu;
mod snake;
mod taboo;

pub fn make_menu() -> impl crate::app::App {
    let mut menu = crate::menu::MenuApp::default();
    menu.register_app::<crate::snake::SnakeApp, _>("Snake");
    menu.register_app::<crate::taboo::TabooApp, _>("Taboo");
    menu.register_app::<crate::distance_integrator::DistanceIntegrator, _>("Distance Integrator");
    menu
}
