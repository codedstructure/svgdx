use svgd::Transformer;

fn main() {
    let mut t = Transformer::new("blob.svg");
    t.transform();
}
