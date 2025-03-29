use operational_transform::OperationSeq;

fn main() {
    let mut a = OperationSeq::default();
    a.insert("abc");
    let mut b = OperationSeq::default();
    b.retain(3);
    b.insert("def");
    let after_a = a.apply("").unwrap();
    let after_b = b.apply(&after_a).unwrap();
    let c = a.compose(&b).unwrap();
    let after_ab = a.compose(&b).unwrap().apply("").unwrap();
    assert_eq!(after_ab, after_b);
}
