# OSH-Lab01-Report

## boot

1. Makefile

   根据Makefile中内容，通过`make bootloader.img`可实现编译，`make qemu`可在qemu中运行。

2. Problems

   * `jmp $` 又是在干什么？

     进入死循环，便于学习过程中调试

   * `boot.asm` 文件前侧的 `org 0x7c00` 有什么用？

     指定了该代码段的初始绝对地址

   * 尝试修改代码，在目前已有的输出中增加一行输出“I am OK!”，样式不限，位置不限，但不能覆盖其他的输出

     在loader.asm`文件中增加

     ```assembly
     log_info IamOK, 7, 4
     ```

     ```assembly
     IamOK: db 'I am OK'
     ```

     输出结果如图

     <img src="report.assets/image-20210422103454352.png" alt="image-20210422103454352" style="zoom:67%;" />

     ## 

     