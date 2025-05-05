use autosurgeon::{Hydrate, Reconcile, hydrate, reconcile};

#[derive(Debug, Clone, Reconcile, Hydrate, PartialEq)]
struct Document {
    content: String,
}

fn main() {
    let mut doc1 = automerge::AutoCommit::new();
    let mut client1 = Document {
        content: String::new(),
    };
    let mut doc2 = doc1.fork().with_actor(automerge::ActorId::random());
    let mut client2 = client1.clone();

    // reconcile(&mut doc1, &client1).unwrap();
    // reconcile(&mut doc2, &client2).unwrap();

    client1.content += "a";
    client2.content += "b";

    reconcile(&mut doc1, &client1).unwrap();
    reconcile(&mut doc2, &client2).unwrap();

    doc1.merge(&mut doc2).unwrap();

    let merged: Document = hydrate(&doc1).unwrap();
    assert_eq!(client1, merged);
    assert_eq!(client2, merged);
}
