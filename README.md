# rust-linked-list

写个链表有那么难吗?

用rust实现链表相关的结构

## 设计思考

* 在 Rust 中，链表的一个核心问题是**所有权**的管理。谁拥有所有权，谁就负责释放内存。
  对于节点来说，每个节点拥有下一个节点的所有权，因此头节点拥有整个链表的所有权。其他情况下，一律使用指针避免所有权的转移
* 虽然用指针会出现**unsafe**操作, 但是它心智负担较低,该用还是得用
* 使用 `Option<NonNull<T>>` 代替 `*mut T`，提供更丰富的语义、编译时检查和更好的可读性
* `Option<NonNull<...>>`、指针都不能在线程间传递=>用新的结构体包裹并实现Send trait
* 对于 `WrapStruct<T>(T)`，`&mut T` 转换为 `*mut T` 可以避免持有不可变引用，允许 WrapStruct 的所有权转移。
* 双向链表在长度为 0 时没有节点，因此头尾指针使用 `Option<...>`。而双锁队列在长度为 0 时有空节点，因此头尾指针不使用
  `Option<...>`，以表示必然有值