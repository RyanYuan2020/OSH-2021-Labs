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

## Init

1. `init.c`

   ```c
   #include <stdio.h>
   #include <unistd.h>
   #include <stdlib.h>
   #include <sys/wait.h>
   #include <sys/types.h>
   #include <sys/stat.h>
   #include <fcntl.h>
   #include <sys/sysmacros.h>
   
   int main()
   {
       
       // Devices
       if (mknod("/dev/ttyS0", S_IFCHR | S_IRUSR | S_IWUSR, makedev(4, 64)) == -1)
       {
           perror("mknod() failed");
       }
       if (mknod("/dev/ttyAMA0", S_IFCHR | S_IRUSR | S_IWUSR, makedev(204, 64)) == -1)
       {
           perror("mknod() failed");
       }
       if (mknod("/dev/fb0", S_IFCHR | S_IRUSR | S_IWUSR, makedev(29, 0)) == -1)
       {
           perror("mknod() failed");
       }
   
       // Run test 1
       int rc1 = fork();
       if (rc1 < 0)
       {
           fprintf(stderr, "fork error\n");
       }
       else if (rc1 == 0)
       {
           if (execl("tools/binary/1", "1", NULL) == -1)
               fprintf(stderr, "execl error1");
       }
       wait(NULL);
   
       // Run test 2
       int rc2 = fork();
       if (rc2 < 0)
       {
           fprintf(stderr, "fork error\n");
       }
       else if (rc2 == 0)
       {
           if (execl("tools/binary/2", "2", NULL) == -1)
               fprintf(stderr, "execl error2");
       }
       wait(NULL);
   
       // Run test 3
       int rc3 = fork();
       if (rc3 < 0)
       {
           fprintf(stderr, "fork error\n");
       }
       else if (rc3 == 0)
       {
           if (execl("tools/binary/3", "3", NULL) == -1)
               fprintf(stderr, "execl error3");
       }
       wait(NULL);
   
       while (1);
       return 0;
   }
   ```

2. 运行测试程序

   ```bash
   qemu-system-aarch64 \
     -kernel linux/arch/arm64/boot/Image \
     -initrd initrd.cpio.gz \
     -dtb tools/boot_utils/bcm2710-rpi-3-b-plus.dtb \
     -M raspi3 -m 1024 \
     -serial stdio \
     -append "rw loglevel=0"
   ```

   <img src="report.assets/image-20210422104429610.png" alt="image-20210422104429610" style="zoom:67%;" />

   