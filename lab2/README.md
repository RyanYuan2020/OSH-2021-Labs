# Report on Lab02: Shell

## Shell

### 实现功能及测试样例

* 所有要求的基础功能

  * Pipe， IO Redirection

    `$ ls --help | grep and | wc > res.txt`

  * 对`SIGINT`信号的相应（`ctrl+C`）

  * 无其他输入接受`EOF`后结束shell（`ctrl+D`）

* 选做内容

  * 基于file descriptor的IO重定向

    `$ grep char 0< file.txt 1>&2`

    `$ grep char <file1.txt 1>file2.txt`

  * here document

    ```bash
    $ wc << delimiter
    > this
    > is
    > a
    > test
    > delimiter
    ```

### 编译方式

本项目使用`cargo`管理。在`lab2/shell`目录下使用`cargo build`可编译本工程；`cargo run`运行。

## 使用 `strace `工具追踪系统调用

`strace target/debug/shell`

1. `mmap`

   将文件内容映射到进程的虚拟内存中，从而可以通过访问内存的方式读取文件内容。

2. `mprotect`

   改变指定虚拟内存范围内的页的权限，如读、写、执行等。

3. `brk`

   改变`program break`的地址。`program break`的地址是本进程`data`段中未初始化部分之后的第一个地址。通过调整`program break`可以分配、释放内存。

