# Report on Lab02: Shell

## Shell

Written in `rust`.  

### 实现功能及测试样例

* 所有要求的基础功能

  * Pipe， IO Redirection 及其组合

    `$ ls --help | grep and | wc > res.txt`

  * 对`SIGINT`信号的响应（`ctrl+C`）

  * 无其他输入时，接收`EOF`后结束*shell*（`ctrl+D`）

* 选做内容

  * 基于*file descriptor*的IO重定向

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
    
  * TCP IO Redirection
  
    1. `server.c`
  
       `lab2/server/server.c`是使用`C`编写的一个简单的服务器程序。
  
       此程序`fork`2个子进程：`Recv Server`, `Send Server`: 
  
       * `Recv Server`
  
         持续接受向**`127.0.0.1:8887`**端口写入的数据，并将数据打印至终端
  
       * `Send Server`
  
         持续向**`127.0.0.1:8888`**端口发送`Hello World\nLine2\nLine3`消息。
    
       `lab2/server/server`是由`gcc server.c -o server`生成的可执行文件。
    
    2. 测试实例
    
       1. 另打开一个终端，启动`server`
  
          ```bash
          $ ./server
          Recv Server: Listen port 8887
          Recv Server: Waiting for client connection
          Send Server: Listen port 8888
          Send Server: Waiting for client connection
          ```
  
       2. 在*shell*中执行`ls`, `wc`命令，并分别将输出、输入重定向至`server`的接受、发送端口
  
           ```bash
           $ ls > /dev/tcp/127.0.0.1/8887
           $ wc < /dev/tcp/127.0.0.1/8888
                 3       4      25
           ```
    
       3. 在`server`所在终端中预期看到如下信息
    
           ```bash
           Recv Server: Client is successfully connected
           Recv Server: Data received from client：Cargo.lock
           Cargo.toml
           src
           target
           
           Recv Server: Waiting for client connection
           Send Server: Client is successfully connected
           Send Server: Data sent to client：Hello World!
           Line2
           Line3
           
           Send Server: Waiting for client connection
           ```
       
       说明：
       
       * 本例中`ls`的结果为
       
         ```bash
         Cargo.lock
         Cargo.toml
         src
         target
         ```
       
       * `wc`的结果等价于
       
         ```bash
         $ wc << EOF
         > Hello World!
         > Line2
         > Line3
         > EOF
          3  4 25
         ```

### 编译方式

本项目使用`cargo`管理。在`lab2/shell`目录下使用`cargo build --release`可编译本工程；`cargo run`运行。

## 使用 `strace `工具追踪系统调用

`strace target/debug/shell`

1. `mmap`

   将文件内容映射到进程的虚拟内存中，从而可以通过访问内存的方式读取文件内容。

2. `mprotect`

   改变指定虚拟内存范围内的页的权限，如读、写、执行等。

3. `brk`

   改变`program break`的地址。`program break`的地址是本进程`data`段中未初始化部分之后的第一个地址。通过调整`program break`可以分配、释放内存。
