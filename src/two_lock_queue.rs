//! 双锁队列实现
//! 
//! 这个模块提供了一个线程安全的双锁队列实现。队列使用两个互斥锁分别保护头尾节点，
//! 允许并发的入队和出队操作。
//!
//! # 设计特点
//! 
//! - **双锁设计**：使用独立的互斥锁保护队列的头部和尾部，减少线程竞争
//! - **空节点**：队列始终保持一个空节点，简化并发操作
//! - **原子计数**：使用原子操作追踪队列长度
//!
//! # 内存布局
//! ```text
//! head (Mutex)          tail (Mutex)
//!      |                     |
//!      v                     v
//!    +---+    +---+    +---+
//!    |   |--->|   |--->|   |
//!    +---+    +---+    +---+
//!   (empty)   data    data
//! ```

use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Debug)]
struct Node<T> {
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

unsafe impl<T: Send> Send for TwoLockQueue<T> {}
unsafe impl<T: Send> Sync for TwoLockQueue<T> {}

pub struct TwoLockQueue<T> {
    head: Mutex<Box<Node<T>>>,
    tail: Mutex<NonNull<Node<T>>>,
    len: AtomicUsize,
}

impl<T> TwoLockQueue<T> {
    pub fn new() -> Self {
        let mut head = Box::new(Node::empty());
        let tail = NonNull::from(&mut *head);
        
        Self {
            head: Mutex::new(head),
            tail: Mutex::new(tail),
            len: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, val: T) {
        let mut new_node = Box::new(Node::new(val));
        let new_ptr = NonNull::from(&mut *new_node);

        let mut tail = self.tail.lock().unwrap();
        
        unsafe {
            tail.as_mut().next = Some(new_node);
            *tail = new_ptr;
        }

        self.len.fetch_add(1, Ordering::SeqCst);
    }

    pub fn pop(&self) -> Option<T> {
        let mut head = self.head.lock().unwrap();

        if self.len.load(Ordering::Acquire) == 0 {
            return None;
        }

        *head = head.next.take().unwrap();
        self.len.fetch_sub(1, Ordering::SeqCst);

        head.data.take()
    }

    pub fn len(&self) -> usize {
        self.len.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;
    use rand::Rng;
    use super::*;

    #[test]
    fn test_empty_queue() {
        let queue: TwoLockQueue<i32> = TwoLockQueue::new();
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn test_single_thread_operations() {
        let queue = TwoLockQueue::new();
        
        // 基本操作测试
        queue.push(1);
        queue.push(2);
        queue.push(3);
        assert_eq!(queue.len(), 3);
        
        // FIFO顺序测试
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), Some(3));
        assert_eq!(queue.pop(), None);
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_multiple_producers() {
        let queue = Arc::new(TwoLockQueue::new());
        let mut handles = vec![];
        
        // 10个生产者线程
        for i in 0..10 {
            let queue = queue.clone();
            handles.push(thread::spawn(move || {
                for j in 0..100 {
                    queue.push(i * 100 + j);
                }
            }));
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        assert_eq!(queue.len(), 1000);
    }

    #[test]
    fn test_multiple_consumers() {
        let queue = Arc::new(TwoLockQueue::new());
        
        // 先填充数据
        for i in 0..1000 {
            queue.push(i);
        }
        
        let mut handles = vec![];
        let counter = Arc::new(AtomicUsize::new(0));
        
        // 5个消费者线程
        for _ in 0..5 {
            let queue = queue.clone();
            let counter = counter.clone();
            handles.push(thread::spawn(move || {
                while let Some(_) = queue.pop() {
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        assert_eq!(counter.load(Ordering::SeqCst), 1000);
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_producers_consumers() {
        let queue = Arc::new(TwoLockQueue::new());
        let mut handles = vec![];
        let total_items = Arc::new(AtomicUsize::new(0));
        
        // 生产者线程
        for _ in 0..3 {
            let queue = queue.clone();
            handles.push(thread::spawn(move || {
                for i in 0..100 {
                    queue.push(i);
                    thread::sleep(Duration::from_micros(1));
                }
            }));
        }
        
        // 消费者线程
        for _ in 0..2 {
            let queue = queue.clone();
            let total_items = total_items.clone();
            handles.push(thread::spawn(move || {
                loop {
                    if let Some(_) = queue.pop() {
                        total_items.fetch_add(1, Ordering::SeqCst);
                    } else if queue.len() == 0 {
                        thread::sleep(Duration::from_millis(1));
                        if queue.len() == 0 {
                            break;
                        }
                    }
                }
            }));
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        assert_eq!(total_items.load(Ordering::SeqCst), 300);
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_stress() {
        let queue = Arc::new(TwoLockQueue::new());
        let mut handles = vec![];
        let ops_count = Arc::new(AtomicUsize::new(0));
        
        // 压力测试：多个线程同时进行推入和弹出操作
        for _ in 0..8 {
            let queue = queue.clone();
            let ops_count = ops_count.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..1000 {
                    if rand::random() {
                        queue.push(1);
                    } else if queue.pop().is_some() {
                        ops_count.fetch_add(1, Ordering::SeqCst);
                    }
                }
            }));
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        // 确保队列最终为空
        while queue.pop().is_some() {
            ops_count.fetch_add(1, Ordering::SeqCst);
        }
        
        println!("Total successful operations: {}", ops_count.load(Ordering::SeqCst));
    }

}
