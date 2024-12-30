//! 双向链表的实现
//! 
//! 这个模块提供了一个不考虑线程安全的双向链表实现。链表的每个节点包含一个值和两个指针：
//! 一个指向下一个节点（拥有所有权），另一个指向前一个节点（不拥有所有权）。
//!
//! # 内存布局
//! ```text
//! head             tail
//!  |                |
//!  v                v
//! +-----+    +-----+    +-----+
//! |     |    |     |    |     |
//! | N1  |--->| N2  |--->| N3  |
//! |     |<---|     |<---|     |
//! +-----+    +-----+    +-----+
//! ```
//!
//! # 所有权设计
//! - next 指针：Box<Node<T>> 持有下一个节点的所有权
//! - prev 指针：NonNull<Node<T>> 仅保持对前一个节点的引用
//! - 整体所有权：从 head 开始，通过 next 指针链形成完整的所有权链

use std::ptr::NonNull;
use std::fmt::Display;
use derive_new::new;
#[derive(PartialEq, Eq, Clone, Debug, new)]
struct Node<T> {
    pub val: T,
    // 拥有所有权
    #[new(default)]
    pub next: Option<Box<Node<T>>>,
    // 不拥有所有权
    #[new(default)]
    pub prev: Option<NonNull<Node<T>>>,
}

#[derive(PartialEq, Eq, Clone, Debug, new)]
pub struct DoublyLinkList<T> {
    // 拥有所有权
    #[new(default)]
    head: Option<Box<Node<T>>>,
    // 不拥有所有权
    #[new(default)]
    tail: Option<NonNull<Node<T>>>,
    // 长度
    #[new(default)]
    len: usize,
}

impl<T> DoublyLinkList<T> {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn push_back(&mut self, val: T) {
        let mut new_box = Box::new(Node::new(val));
        new_box.prev = self.tail;  // Copy语义

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

    pub fn push_front(&mut self, val: T) {
        let mut new_box = Box::new(Node::new(val));
        
        match self.head.take() {
            Some(mut old_head) => {
                // 设置新节点为头节点
                old_head.prev = NonNull::new(new_box.as_mut());
                new_box.next = Some(old_head);
                self.head = Some(new_box);
            }
            None => {
                // 空链表
                self.tail = NonNull::new(new_box.as_mut());
                self.head = Some(new_box);
            }
        }
        
        self.len += 1;
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.head.take().map(|old_head| {
            self.len -= 1;
            
            match old_head.next {
                Some(mut next) => {
                    next.prev = None;
                    self.head = Some(next);
                }
                None => {
                    self.tail = None;
                }
            }
            
            old_head.val
        })
    }
}

impl<T: Display> Display for DoublyLinkList<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LinkList [")?;
        
        let mut current = &self.head;
        while let Some(node) = current {
            write!(f, "{}", node.val)?;
            current = &node.next;
            if current.is_some() {
                write!(f, ", ")?;
            }
        }
        
        write!(f, "]")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_new_list() {
        let list: DoublyLinkList<i32> = DoublyLinkList::new();
        assert_eq!(list.len(), 0);
        assert_eq!(list.to_string(), "LinkList []");
    }

    #[test]
    fn test_push_operations() {
        let mut list = DoublyLinkList::new();
        
        // 测试 push_back
        list.push_back(1);
        assert_eq!(list.to_string(), "LinkList [1]");
        
        // 测试 push_front
        list.push_front(2);
        assert_eq!(list.to_string(), "LinkList [2, 1]");
        
        // 混合测试
        list.push_back(3);
        list.push_front(4);
        assert_eq!(list.to_string(), "LinkList [4, 2, 1, 3]");
        assert_eq!(list.len(), 4);
    }

    #[test]
    fn test_pop_operations() {
        let mut list = DoublyLinkList::new();
        
        // 空链表弹出测试
        assert_eq!(list.pop_front(), None);
        assert_eq!(list.pop_back(), None);
        
        // 添加元素
        list.push_back(1);
        list.push_back(2);
        list.push_back(3);
        
        // 从后往前弹出
        assert_eq!(list.pop_back(), Some(3));
        assert_eq!(list.pop_back(), Some(2));
        assert_eq!(list.pop_back(), Some(1));
        assert_eq!(list.pop_back(), None);
        assert_eq!(list.len(), 0);
        
        // 从前往后弹出
        list.push_back(1);
        list.push_back(2);
        list.push_back(3);
        
        assert_eq!(list.pop_front(), Some(1));
        assert_eq!(list.pop_front(), Some(2));
        assert_eq!(list.pop_front(), Some(3));
        assert_eq!(list.pop_front(), None);
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_mixed_operations() {
        let mut list = DoublyLinkList::new();
        
        // 混合操作测试
        list.push_front(1);  // [1]
        list.push_back(2);   // [1, 2]
        list.push_front(3);  // [3, 1, 2]
        assert_eq!(list.pop_back(), Some(2));  // [3, 1]
        list.push_back(4);   // [3, 1, 4]
        assert_eq!(list.pop_front(), Some(3)); // [1, 4]
        
        assert_eq!(list.to_string(), "LinkList [1, 4]");
        assert_eq!(list.len(), 2);
    }
}
