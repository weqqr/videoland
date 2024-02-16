pub trait Pass {
    fn pass(&self);
}

pub struct WorldPass {

}

impl Pass for WorldPass {
    fn pass(&self) {
        unimplemented!()
    }
}
