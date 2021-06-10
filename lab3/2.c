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
#define send_start (is_during_contiguous_send ? send_buffer + offset : send_buffer)
#define MAX_CLIENT_NUM 32

typedef struct Client
{
	int valid;
	int fd_send;
	pthread_mutex_t mutex;
	pthread_t thread;
} Client;

Client clients[MAX_CLIENT_NUM];
int valid_clients_num = 0;

pthread_mutex_t ClientsMutex = PTHREAD_MUTEX_INITIALIZER;

void *handle_chat(void *data);

void init_clients()
{
	for (size_t i = 0; i < MAX_CLIENT_NUM; i++)
		clients[i].valid = 0;
}

void destroy_client(Client *obj)
{
	pthread_mutex_lock(&ClientsMutex);
	obj->valid = 0;
	close(obj->fd_send);
	valid_clients_num--;
	pthread_mutex_destroy(&obj->mutex);
	pthread_mutex_unlock(&ClientsMutex);
	printf("Client left, %d in total\n", valid_clients_num);
}

int add_client(int fd)
{
	int index = -1;
	pthread_mutex_lock(&ClientsMutex);
	for (size_t i = 0; i < MAX_CLIENT_NUM; i++)
	{
		if (!clients[i].valid)
		{
			index = i;
			break;
		}
	}
	if (index < 0)
	{
		pthread_mutex_unlock(&ClientsMutex);
		return -1;
	}
	clients[index].valid = 1;
	clients[index].fd_send = fd;
	valid_clients_num++;
	pthread_mutex_unlock(&ClientsMutex);
	pthread_mutex_init(&clients[index].mutex, NULL);
	pthread_create(&clients[index].thread, NULL, handle_chat, clients + index);
	printf("New client entered, %d in total\n", valid_clients_num);

	return index;
}

ssize_t send_to_all(Client *client, const void *buf, size_t n)
{
	int size = 0;
	for (int i = 0; i < MAX_CLIENT_NUM; i++)
	{
		if (clients[i].valid && (clients + i) != client)
		{
			pthread_mutex_lock(&clients[i].mutex);
			int tmp_size = send(clients[i].fd_send, buf, n, 0);
			pthread_mutex_unlock(&clients[i].mutex);
			printf("Send %d bytes to client %d\n", tmp_size, i);
			if (size == 0)
			{
				size = tmp_size;
			}
			else if (tmp_size != size)
			{
				perror("Inconsistent size of data sent");
			}
		}
	}
	return size;
}

ssize_t receive(Client *client, void *buf, size_t n)
{
	int size = recv(client->fd_send, buf, n, 0);
	return size;
}

void *handle_chat(void *args)
{
	Client *client = (Client *)args;

	char recv_buffer[buffer_size + 32] = {0};
	int recv_buffer_index = 0;
	char send_buffer[buffer_size + 32] = "Message:";
	const int offset = strlen(send_buffer);
	int send_buffer_index = offset;
	ssize_t len;

	int is_during_contiguous_send = 0;
	int is_during_contiguous_recv = 0;

	while (1)
	{
		recv_buffer_index = 0;
		memset(recv_buffer, 0, buffer_size + 32);
		len = receive(client, recv_buffer, buffer_size);
		if (!is_during_contiguous_recv && strcmp(recv_buffer, "$EXIT\n") == 0)
			break;
		if (len <= 0) // received nothing, terminate.
			break;
		while (1)
		{
			if (recv_buffer_index >= buffer_size)
			{
				printf("Massive data received\n");
				is_during_contiguous_recv = 1;
				break;
			}
			if (send_buffer_index >= buffer_size)
			{
				printf("Massive data sent\n");
				send_to_all(client, send_start, send_buffer_index);
				is_during_contiguous_send = 1;
				send_buffer_index = offset;
			}
			if (is_com_char(recv_buffer[recv_buffer_index]))
			{
				send_buffer[send_buffer_index++] = recv_buffer[recv_buffer_index++];
			}
			else if (recv_buffer[recv_buffer_index] == '\n')
			{
				send_buffer[send_buffer_index] = '\n';
				send_to_all(client, send_start, send_buffer_index + 1);
				is_during_contiguous_send = 0;
				send_buffer_index = offset;
				if (recv_buffer_index >= buffer_size - 1)
				{
					is_during_contiguous_recv = 0;
					break;
				}
				else
					recv_buffer_index++;
			}
			else // EOF
			{
				if (send_buffer_index != offset)
					send_to_all(client, send_start, send_buffer_index);
				is_during_contiguous_recv = 0;
				break;
			}
		}
	}
	destroy_client(client);
	return NULL;
}

int main(int argc, char **argv)
{
	int port = atoi(argv[1]);
	int sockfd;
	if ((sockfd = socket(AF_INET, SOCK_STREAM, 0)) == 0)
	{
		perror("socket");
		return 1;
	}
	struct sockaddr_in addr;
	addr.sin_family = AF_INET;
	addr.sin_addr.s_addr = INADDR_ANY;
	addr.sin_port = htons(port);
	socklen_t addr_len = sizeof(addr);
	if (bind(sockfd, (struct sockaddr *)&addr, sizeof(addr)))
	{
		perror("bind");
		return 1;
	}
	if (listen(sockfd, MAX_CLIENT_NUM))
	{
		perror("listen");
		return 1;
	}
	init_clients();
	do
	{
		int fd = accept(sockfd, NULL, NULL);
		if (fd == -1)
		{
			perror("accept");
			return 0;
		}
		add_client(fd);
	} while (valid_clients_num);
	return 0;
}