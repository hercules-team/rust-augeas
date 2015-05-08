extern crate augeas;

use augeas::{Augeas,AugFlag};

fn main() {
    let aug = Augeas::new("tests/test_root", "", AugFlag::None).unwrap();
    let username = "root";

    println!("Infos about '{}':", username);

    let info_nodes = aug.matches("etc/passwd/root/*").unwrap();

    for node in info_nodes.iter() {
        let label = aug.label(&node).unwrap();
        let value = aug.get(&node).unwrap();

        if let (Some(label), Some(value)) = (label, value) {
            println!("{} = {}", label, value)
        }
    }
}
