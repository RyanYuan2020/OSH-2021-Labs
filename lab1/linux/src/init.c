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

    while (1)
        ;

    return 0;
}
