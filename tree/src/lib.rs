use std::mem;

#[derive(PartialEq, Eq, Clone)]
pub struct RoseTree<T, E> {
    pub val: T,
    pub children: Vec<(E, RoseTree<T, E>)>,
}
impl<T, E> RoseTree<T, E> {
    pub fn singleton(val: T) -> Self {
        RoseTree {
            val,
            children: Vec::new(),
        }
    }
}

pub struct Zipper<T, E> {
    frames: Vec<ZipperFrame<T, E>>,
    pub focus: RoseTree<T, E>,
}

struct ZipperFrame<T, E> {
    val: T,
    edge: E,
    left: Vec<(E, RoseTree<T, E>)>,
    right: Vec<(E, RoseTree<T, E>)>, //is reversed
}

impl<T, E> Zipper<T, E> {
    pub fn new(tree: RoseTree<T, E>) -> Self {
        Zipper {
            frames: Vec::new(),
            focus: tree,
        }
    }
    pub fn get_focus_val(&self) -> &T {
        &self.focus.val
    }
    pub fn get_focus_val_mut(&mut self) -> &mut T {
        &mut self.focus.val
    }
    pub fn down(&mut self, i: usize) -> Result<(), &str> {
        if i >= self.focus.children.len() {
            return Err("Index out of bounds");
        }
        let mut left = mem::replace(&mut self.focus.children, Vec::new());
        let new_focus;
        let edge;
        let right;
        {
            let mut left_tail = left.drain(i..);
            let (edge_temp, new_focus_temp) = left_tail.next().expect("Index out of bounds");
            edge = edge_temp;
            new_focus = new_focus_temp;
            right = left_tail.rev().collect();
        }
        let old_focus = mem::replace(&mut self.focus, new_focus);
        self.frames.push(ZipperFrame {
            val: old_focus.val,
            edge,
            left,
            right,
        });
        return Ok(());
    }
    pub fn up(&mut self) -> Result<(), &str> {
        self.frames
            .pop()
            .map(
                |ZipperFrame {
                     val,
                     edge,
                     mut left,
                     right,
                 }| {
                    let old_focus = mem::replace(
                        &mut self.focus,
                        RoseTree {
                            val,
                            children: Vec::new(),
                        },
                    );
                    left.push((edge, old_focus));
                    left.extend(right.into_iter().rev());
                    self.focus.children = left;
                },
            )
            .ok_or("Already at top of zipper")
    }
    pub fn left(&mut self) -> Result<(), &str> {
        let focus = &mut self.focus;
        self.frames
            .get_mut(0)
            .and_then(
                |&mut ZipperFrame {
                     ref mut left,
                     ref mut right,
                     ref mut edge,
                     ..
                 }| {
                    left.pop().map(|(new_edge, new_focus)| {
                        let old_focus = mem::replace(focus, new_focus);
                        let old_edge = mem::replace(edge, new_edge);
                        right.push((old_edge, old_focus));
                    })
                },
            )
            .ok_or("Nothing to the left")
    }
    pub fn right(&mut self) -> Result<(), &str> {
        let focus = &mut self.focus;
        self.frames
            .get_mut(0)
            .and_then(
                |&mut ZipperFrame {
                     ref mut left,
                     ref mut right,
                     ref mut edge,
                     ..
                 }| {
                    right.pop().map(|(new_edge, new_focus)| {
                        let old_focus = mem::replace(focus, new_focus);
                        let old_edge = mem::replace(edge, new_edge);
                        left.push((old_edge, old_focus));
                    })
                },
            )
            .ok_or("Nothing to the right")
    }
    pub fn rezip(mut self) -> RoseTree<T, E> {
        while let Ok(()) = self.up() {}
        return self.focus;
    }
    pub fn push(&mut self, x: T, edge: E) {
        let left = mem::replace(&mut self.focus.children, Vec::new());
        let new_focus = RoseTree::singleton(x);
        let old_focus = mem::replace(&mut self.focus, new_focus);
        self.frames.push(ZipperFrame {
            val: old_focus.val,
            edge,
            left,
            right: Vec::new(),
        });
    }
}
