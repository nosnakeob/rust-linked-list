mod doubly;
mod two_lock_queue;

// 重新导出数据结构供外部使用
pub use doubly::DoublyLinkList;
pub use two_lock_queue::TwoLockQueue;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        println!("it works");
    }
}
