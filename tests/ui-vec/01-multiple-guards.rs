use triple_r::ReusableVec;

fn main() {
    let mut vec = ReusableVec::<i32>::default();
    let _guard1 = vec.recycle::<i32>();
    let _guard2 = vec.recycle::<i32>();
}
