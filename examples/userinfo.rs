extern crate augeas;

use augeas::{Augeas, Flags};

fn main() {
    let aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();
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
