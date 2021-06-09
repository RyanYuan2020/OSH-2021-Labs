#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <pthread.h>
#include <memory.h>

#define is_com_char(x) ((x) != '\n' && (x) != 0)
#define buffer_size (1 << 10)
#define send_start (is_during_contiguos_send ? send_buffer + offset : send_buffer)

struct Pipe
{
	int fd_send;
	int fd_recv;
};

void *handle_chat(void *data)
{
	struct Pipe *pipe = (struct Pipe *)data;
	char recv_buffer[buffer_size + 32] = {0};
	int recv_buffer_index = 0;
	char send_buffer[buffer_size + 32] = "Message:";
	const int offset = strlen(send_buffer);
	int send_buffer_index = offset;
	ssize_t len;

	int is_during_contiguos_send = 0;

	while (1)
	{
		recv_buffer_index = 0;
		memset(recv_buffer, 0, buffer_size + 32);
		len = recv(pipe->fd_send, recv_buffer, buffer_size, 0);
		if (len <= 0) // received nothing, terminate.
			break;
		while (1)
		{
			if (recv_buffer_index >= buffer_size)
			{
				printf("Massive data received\n");
				break;
			}
			if (send_buffer_index >= buffer_size)
			{
				printf("Massive data sent\n");
				send(pipe->fd_recv, send_start, send_buffer_index, 0);
				is_during_contiguos_send = 1;
				send_buffer_index = offset;
			}
			if (is_com_char(recv_buffer[recv_buffer_index]))
			{
				send_buffer[send_buffer_index++] = recv_buffer[recv_buffer_index++];
			}
			else if (recv_buffer[recv_buffer_index] == '\n')
			{
				send_buffer[send_buffer_index] = '\n';
				send(pipe->fd_recv, send_start, send_buffer_index + 1, 0);
				is_during_contiguos_send = 0;
				send_buffer_index = offset;
				recv_buffer_index++;
			}
			else // EOF
			{
				if (send_buffer_index != offset)
					send(pipe->fd_recv, send_start, send_buffer_index, 0);
				break;
			}
		}
	}
	return NULL;
}

int main(int argc, char **argv)
{
	int port = atoi(argv[1]);
	int fd;
	if ((fd = socket(AF_INET, SOCK_STREAM, 0)) == 0)
	{
		perror("socket");
		return 1;
	}
	struct sockaddr_in addr;
	addr.sin_family = AF_INET;
	addr.sin_addr.s_addr = INADDR_ANY;
	addr.sin_port = htons(port);
	socklen_t addr_len = sizeof(addr);
	if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)))
	{
		perror("bind");
		return 1;
	}
	if (listen(fd, 2))
	{
		perror("listen");
		return 1;
	}
	int fd1 = accept(fd, NULL, NULL);
	int fd2 = accept(fd, NULL, NULL);
	if (fd1 == -1 || fd2 == -1)
	{
		perror("accept");
		return 1;
	}
	pthread_t thread1, thread2;
	struct Pipe pipe1;
	struct Pipe pipe2;
	pipe1.fd_send = fd1;
	pipe1.fd_recv = fd2;
	pipe2.fd_send = fd2;
	pipe2.fd_recv = fd1;
	pthread_create(&thread1, NULL, handle_chat, (void *)&pipe1);
	pthread_create(&thread2, NULL, handle_chat, (void *)&pipe2);
	pthread_join(thread1, NULL);
	pthread_join(thread2, NULL);
	return 0;
}