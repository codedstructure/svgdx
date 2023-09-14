use svgd::Transformer;

fn main() {
    let mut t = Transformer::new("boxes-tbox.svg");
    t.transform();
}
