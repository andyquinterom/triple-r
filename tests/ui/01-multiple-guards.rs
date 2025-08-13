use triple_r::ReusableHashMap;

fn main() {
    let mut map = ReusableHashMap::<&'static str, &'static str>::default();
    let _guard1 = map.recycle::<&str, &str>();
    let _guard2 = map.recycle::<&str, &str>();
}
