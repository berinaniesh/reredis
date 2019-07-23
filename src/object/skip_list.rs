use std::rc::Rc;
use crate::object::{RobjPtr, Robj, RobjType, Sds};
use rand::prelude::*;
use std::iter::Skip;
use core::borrow::{Borrow, BorrowMut};
use std::cell::{Ref, RefCell};

const SKIP_LIST_MAX_LEVEL: usize = 32;

pub struct SkipListLevel {
    forward: Option<Rc<RefCell<SkipListNode>>>,
    span: usize,
}

pub struct SkipListNode {
    obj: Option<RobjPtr>,
    score: f64,
    backward: Option<Rc<RefCell<SkipListNode>>>,
    level: Vec<SkipListLevel>,
}

impl SkipListNode {
    fn new(level: usize, score: f64, obj: Option<RobjPtr>) -> SkipListNode {
        let mut level_vec: Vec<SkipListLevel>
            = Vec::with_capacity(level);

        for _ in 0..level {
            level_vec.push(SkipListLevel {
                forward: None,
                span: 0,
            });
        }

        let mut node = SkipListNode {
            obj: None,
            score,
            backward: None,
            level: level_vec,
        };

        if let Some(p) = obj {
            node.obj = Some(p);
        }

        node
    }

    fn obj_ref(&self) -> &RobjPtr {
        self.obj.as_ref().unwrap()
    }
}

pub struct SkipList {
    header: Rc<RefCell<SkipListNode>>,
    tail: Option<Rc<RefCell<SkipListNode>>>,
    length: usize,
    level: usize,
}

impl SkipList {
    fn new() -> SkipList {
        let mut header =
            SkipListNode::new(SKIP_LIST_MAX_LEVEL, 0.0, None);

        header.backward = None;

        SkipList {
            header: Rc::new(RefCell::new(header)),
            tail: None,
            length: 0,
            level: 1,
        }
    }

    fn random_level() -> usize {
        let mut level = 1usize;
        let mut rng = rand::thread_rng();
        let mut num: usize = rng.gen();

        while ((num & 0xFFFFusize) as f64) < (0.25 * (0xFFFF as f64)) {
            level += 1;
            num = rng.gen();
        }

        if level < SKIP_LIST_MAX_LEVEL {
            return level;
        }
        SKIP_LIST_MAX_LEVEL
    }

    fn insert(&mut self, score: f64, obj: RobjPtr) {
        let mut update: Vec<Option<Rc<RefCell<SkipListNode>>>> =
            Vec::with_capacity(SKIP_LIST_MAX_LEVEL);

        for i in 0..SKIP_LIST_MAX_LEVEL {
            update.push(None)
        }

        let mut rank = [0usize; SKIP_LIST_MAX_LEVEL];

        let mut x = Rc::clone(&self.header);

        for i in (0..self.level).rev() {
            rank[i] = if i == self.level - 1 {
                0
            } else {
                rank[i + 1]
            };


            loop {
                let curr = Rc::clone(&x);
                let this_node = curr.as_ref().borrow();
                if this_node.level[i].forward.is_none() {
                    break;
                }
                let forward = this_node.level[i]
                    .forward.as_ref().unwrap();
                let next_node = forward.as_ref().borrow();
                let next_score = next_node.score.clone();
                let next_obj = next_node.obj.as_ref().unwrap().as_ref().borrow();

                if next_score > score || (next_score == score &&
                    next_obj.string() >= obj.as_ref().borrow().string()) {
                    break;
                }

                rank[i] += this_node.level[i].span;

                x = Rc::clone(forward);
            }

            update[i] = Some(Rc::clone(&x));
        }

        let level = SkipList::random_level();

        if level > self.level {
            for i in self.level..level {
                rank[i] = 0;
                update[i] = Some(Rc::clone(&self.header));
                update[i].as_ref().unwrap()
                    .as_ref().borrow_mut().level[i].span = self.length;
            }

            self.level = level;
        }

        let new_node = Rc::new(
            RefCell::new(
                SkipListNode::new(level, score, Some(obj))
            )
        );
        let curr = new_node.as_ref();

        for i in 0..level {
            let prev = update[i].as_ref().unwrap().as_ref();

            curr.borrow_mut().level[i].forward = match prev.borrow().level[i].forward {
                None => None,
                Some(_) => Some(Rc::clone(prev.borrow().level[i]
                    .forward.as_ref().unwrap())),
            };

            prev.borrow_mut().level[i].forward = Some(Rc::clone(&new_node));

            curr.borrow_mut().level[i].span
                = prev.borrow().level[i].span - (rank[0] - rank[i]);

            prev.borrow_mut().level[i].span = (rank[0] - rank[i]) + 1;
        }

        for i in level..self.level {
            update[i].as_ref().unwrap().as_ref().borrow_mut().level[i].span += 1;
        }

        curr.borrow_mut().backward = if Rc::ptr_eq(
            &self.header, update[0].as_ref().unwrap(),
        ) {
            None
        } else {
            Some(Rc::clone(update[0].as_ref().unwrap()))
        };

        if let Some(e) = curr.borrow().level[0].forward.as_ref() {
            e.as_ref().borrow_mut().backward = Some(Rc::clone(&new_node));
        } else {
            self.tail = Some(Rc::clone(&new_node));
        }

        self.length += 1;

    }
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::object::{Robj, RobjPtr};

    #[test]
    fn create_new_skip_list() {
        let list = SkipList::new();
        assert_eq!(list.length, 0);
        assert_eq!(list.level, 1);
    }

    #[test]
    fn generate_rand_level() {
        let mut levels = vec![0usize; 33];
        for i in 0..100000 {
            let l = SkipList::random_level();
            levels[l] += 1;
        }

        let q = levels.iter().skip(2);
        for p in levels.iter().skip(1).zip(q) {
            assert!(p.0 >= p.1);
        }
    }

    #[test]
    fn simple_insert() {
        let mut list = SkipList::new();
        let o1 = Robj::create_string_object("foo");
        let o2 = Robj::create_string_object("bar");

        list.insert(3.2, o1);
        list.insert(0.2, o2);
    }
}


