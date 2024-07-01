/// 双锁队列

use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;


// 不持有所有权、可以线程间传递所有权
#[derive(Debug)]
struct NodePtr<T>(NonNull<Node<T>>);

impl<T> NodePtr<T> {
    fn as_ptr(&self) -> *mut Node<T> {
        self.0.as_ptr()
    }

    fn as_mut(&mut self) -> &mut Node<T> {
        unsafe { self.0.as_mut() }
    }

    fn as_ref(&self) -> &Node<T> {
        unsafe { self.0.as_ref() }
    }
}

// 借用box转换
impl<T> From<&mut Box<Node<T>>> for NodePtr<T> {
    fn from(value: &mut Box<Node<T>>) -> Self {
        NodePtr(NonNull::from(value.as_mut()))
    }
}

// 裸指针转换
impl<T> From<*mut Node<T>> for NodePtr<T> {
    fn from(value: *mut Node<T>) -> Self {
        unsafe { NodePtr(NonNull::new_unchecked(value)) }
    }
}

unsafe impl<T> Send for NodePtr<T> {}


#[derive(Debug)]
struct Node<T> {
    // 可能为空节点
    data: Option<T>,
    next: Option<Box<Node<T>>>,
}

impl<T> Node<T> {
    fn new(data: T) -> Self {
        Self { data: Some(data), next: None }
    }

    fn empty() -> Self {
        Self { data: None, next: None }
    }
}

struct TwoLockQueue<T> {
    // 所有权
    head: Mutex<Box<Node<T>>>,
    tail: Mutex<NodePtr<T>>,
    len: AtomicUsize,
}

impl<T> TwoLockQueue<T> {
    fn new() -> Self {
        let mut head = Box::new(Node::empty());
        let tail = NodePtr::from(&mut head);
        Self {
            head: Mutex::new(head),
            tail: Mutex::new(tail),
            len: AtomicUsize::new(0),
        }
    }

    fn push(&self, val: T) {
        let mut new_node = Box::new(Node::new(val));
        // 借用变指针, 即使new_node所有权转移, 也不会影响指针
        let new_p = new_node.as_mut() as *mut Node<T>;

        let mut tail = self.tail.lock().unwrap();

        unsafe {
            tail.as_mut().next = Some(new_node);
            *tail = NodePtr::from(new_p);
        }

        self.len.fetch_add(1, Ordering::SeqCst);
    }

    fn pop(&self) -> Option<T> {
        let mut head = self.head.lock().unwrap();

        if self.len.load(Ordering::Acquire) == 0 {
            return None;
        }

        *head = head.next.take().unwrap();
        self.len.fetch_sub(1, Ordering::SeqCst);

        head.data.take()
    }

    fn len(&self) -> usize {
        self.len.load(Ordering::Acquire)
    }
}


#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use super::*;

    #[test]
    fn t_node() {
        let mut head = Node::empty();
        let cur = head.next.insert(Box::new(Node::new(1)));
        let tail = cur.next.insert(Box::new(Node::new(2)));

        let tail_p = NonNull::new(tail.as_mut());

        println!("{:?}", head);
        println!("{:?}", unsafe { tail_p.unwrap().as_ref() });
    }

    #[test]
    fn t_node_ptr() {
        let mut head = Box::new(Node::new(1));
        let head_p = head.as_mut() as *mut Node<i32>;

        let tail_p1 = NodePtr::from(&mut head);
        let tail_p2 = NodePtr::from(head_p);

        thread::spawn(move || {
            // println!("{:p}", tail_p.as_ptr());
            assert!(!tail_p1.as_ptr().is_null());
            assert!(!tail_p2.as_ptr().is_null());
        }).join().unwrap();

        // println!("{:p}", head);
    }

    #[test]
    fn t_two_lock_link_list() {
        let mut list: TwoLockQueue<i32> = TwoLockQueue::new();

        list.push(1);
        list.push(2);
        list.push(3);

        assert_eq!(list.len(), 3);

        assert_eq!(list.pop(), Some(1));
        assert_eq!(list.pop(), Some(2));
        assert_eq!(list.pop(), Some(3));
        assert_eq!(list.pop(), None);

        assert_eq!(list.len(), 0);
    }

    #[test]
    fn t_two_lock_link_list_thread() {
        let mut list = Arc::new(TwoLockQueue::new());

        let mut handles = vec![];
        for i in 0..10 {
            let list = list.clone();
            handles.push(thread::spawn(move || {
                list.push(i);
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(list.len(), 10);

        while let Some(val) = list.pop() {
            assert!(val >= 0 && val < 10);
            // println!("pop: {}", val);
        }

        assert_eq!(list.pop(), None);

        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_two_lock_link_list_multi_thread() {
        let list = Arc::new(TwoLockQueue::new());
        let mut handles = vec![];

        for i in 0..10 {
            let list = list.clone();
            handles.push(thread::spawn(move || {
                println!("push: {}", i);
                list.push(i);
            }));
        }

        for _ in 0..10 {
            let list = list.clone();
            handles.push(thread::spawn(move || {
                loop {
                    if let Some(val) = list.pop() {
                        println!("pop: {}", val);
                        return;
                    }
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(list.len(), 0);
    }
}
