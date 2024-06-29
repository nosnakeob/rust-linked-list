/// 不考虑线程安全的双向链表
///
/// 结构:
/// ``` txt
/// -------------                -------------
/// |  Node<T>  |   ---------->  |  Node<T>  |
/// |           |   <- - - - -   |           |
/// -------------                -------------
/// ```
/// next持有所有权, prev不持有所有权 => 对于链表,头部持有整个的所有权

use std::ptr::NonNull;

#[derive(PartialEq, Eq, Clone, Debug)]
struct Node<T> {
    pub val: T,
    // 拥有所有权
    pub next: Option<Box<Node<T>>>,
    // 不拥有所有权
    pub prev: Option<NonNull<Node<T>>>,
}

impl<T> Node<T> {
    pub fn new(val: T) -> Self {
        Self {
            val,
            next: None,
            prev: None,
        }
    }
}

struct LinkList<T> {
    // 拥有所有权
    head: Option<Box<Node<T>>>,
    // 不拥有所有权
    tail: Option<NonNull<Node<T>>>,
    len: usize,
}

impl<T> LinkList<T> {
    pub fn new() -> Self {
        Self {
            head: None,
            tail: None,
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn push_back(&mut self, val: T) {
        let mut new_box = Box::new(Node::new(val));
        new_box.prev = self.tail;

        // 转为裸指针 避免borrow check
        let p = new_box.as_mut() as *mut Node<T>;

        if let Some(mut tail) = self.tail {
            unsafe {
                tail.as_mut().next = Some(new_box);
            }
        } else {
            self.head = Some(new_box);
        }

        self.tail = NonNull::new(p);
        self.len += 1;
    }

    pub fn pop_back(&mut self) -> Option<T> {
        self.tail.map(|mut tail| unsafe {
            self.len -= 1;

            // prev -> node
            if let Some(mut prev) = tail.as_mut().prev {
                let node = prev.as_mut().next.take().unwrap();
                self.tail = Some(prev);
                node.val
                // head = node
            } else {
                let node = self.head.take().unwrap();
                self.tail = None;
                node.val
            }
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_link_list() {
        let mut list = LinkList::new();

        list.push_back(1);
        list.push_back(2);
        list.push_back(3);

        assert_eq!(list.len(), 3);

        assert_eq!(list.pop_back(), Some(3));
        assert_eq!(list.pop_back(), Some(2));
        assert_eq!(list.pop_back(), Some(1));
        assert_eq!(list.pop_back(), None);

        assert_eq!(list.len(), 0);
    }
}
