extern crate embed_resource;

fn main() {

    // #[cfg(debug_assertions)]
    embed_resource::compile("debug.rc");

    
    // #[cfg(not(debug_assertions))]
    // embed_resource::compile("release.rc");
}
