use std::mem;

#[derive(PartialEq, Eq, Clone)]
pub struct RoseTree<T> {
    pub val : T,
    pub children : Vec<RoseTree<T>>
}

pub struct Zipper<T> {
    frames : Vec<ZipperFrame<T>>,
    pub focus : RoseTree<T>
}

struct ZipperFrame<T> {
    val : T,
    left : Vec<RoseTree<T>>,
    right : Vec<RoseTree<T>> //is reversed
}

impl<T> Zipper<T> {
    pub fn new(tree : RoseTree<T>) -> Self {
        Zipper{
            frames : Vec::new(),
            focus : tree
        }
    }
    pub fn down(& mut self, i : usize) -> Result<(),&str> {
        if i >= self.focus.children.len(){
            return Err("Index out of bounds");
        }
        let mut left = mem::replace(&mut self.focus.children, Vec::new());
        let new_focus;
        let right;
        {
            let mut left_tail = left.drain(i..);
            new_focus = left_tail.next().expect("index out of bounds");
            right = left_tail.rev().collect();
        }
        let old_focus = mem::replace(&mut self.focus, new_focus);
        self.frames.push(ZipperFrame {val : old_focus.val, left:left, right:right});
        return Ok(());
    }
    pub fn up(& mut self) -> Result<(), &str> {
        self.frames.pop().map(|ZipperFrame{val, mut left, right}| {
            let old_focus = mem::replace(&mut self.focus, RoseTree{val, children : Vec::new()});
            left.push(old_focus);
            left.extend(right.into_iter().rev());
            self.focus.children = left;
        }).ok_or("Already at top of zipper")
    }
    pub fn left(& mut self) -> Result<(), &str> {
        let focus = & mut self.focus;
        self.frames.get_mut(0).and_then(|& mut ZipperFrame{ref mut left, ref mut right,..}|
                                        left.pop().map(|new_focus| {
            let old_focus = mem::replace(focus, new_focus);
            right.push(old_focus);
        })).ok_or("Nothing to the left")
    }
    pub fn right(& mut self) -> Result<(), &str> {
        let focus = & mut self.focus;
        self.frames.get_mut(0).and_then(|& mut ZipperFrame{ref mut left, ref mut right,..}|
                                        right.pop().map(|new_focus| {
            let old_focus = mem::replace(focus, new_focus);
            left.push(old_focus);
        })).ok_or("Nothing to the right")
    }
    pub fn rezip(mut self) -> RoseTree<T> {
        while let Ok(()) = self.up(){}
        return self.focus;
    }
}
