extern crate augeas;

use augeas::{Augeas,AugFlags};

fn main() {
    let aug = Augeas::new("tests/test_root", "", AugFlags::None);
    let username = "root";

    println!("Infos about '{}':", username);

    let info_nodes = aug.matches("etc/passwd/root/*");

    for node in info_nodes.iter() {
        let label = aug.label(node.as_slice());
        let value = aug.get(node.as_slice());

        if let (Some(label), Some(value)) = (label, value) {
            println!("{} = {}", label, value)
        }
    }
}
