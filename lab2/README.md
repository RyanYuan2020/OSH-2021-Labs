# Report on Lab02: Shell

## 使用 `strace `工具追踪系统调用

`strace target/debug/shell`

1. `mmap`

   将文件内容映射到进程的虚拟内存中，从而可以通过访问内存的方式读取文件内容。

2. `mprotect`

   改变指定虚拟内存范围内的页的权限，如读、写、执行等。

3. `brk`

   改变`program break`的地址。`program break`的地址是本进程`data`段中未初始化部分之后的第一个地址。通过调整`program break`可以分配、释放内存。

