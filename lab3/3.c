#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/socket.h>
#include <sys/select.h>
#include <sys/time.h>
#include <netinet/in.h>

#define is_com_char(x) ((x) != '\n' && (x) != 0)
#define buffer_size (1 << 10)
#define NICK_NAME_MAX_LEN 32
#define PADDING 16
#define send_start (is_during_contiguous_send ? send_buffer + offset : send_buffer)
#define send_len (is_during_contiguous_send ? send_buffer_index - offset : send_buffer_index)
#define MAX_CLIENT_NUM 32
#define for_each_valid_client(iter)                                            \
	for (Client * (iter) = clients; (iter)-clients < MAX_CLIENT_NUM; (iter)++) \
		if ((iter)->valid)
#define send_start (is_during_contiguous_send ? send_buffer + offset : send_buffer)

typedef struct Client
{
	int valid;
	int fd_send;
	char nick_name[NICK_NAME_MAX_LEN];
} Client;

Client clients[MAX_CLIENT_NUM];
int valid_clients_num = 0;

void init_clients()
{
	for (size_t i = 0; i < MAX_CLIENT_NUM; i++)
	{
		clients[i].valid = 0;
		sprintf(clients[i].nick_name, "%ld", i);
	}
}

void destroy_client(Client *obj)
{
	obj->valid = 0;
	close(obj->fd_send);
	valid_clients_num--;
	printf("Client left, %d in total\n", valid_clients_num);
}

int add_client(int fd)
{
	int index = -1;
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
		return -1;
	}
	fcntl(fd, F_SETFL, fcntl(fd, F_GETFL, 0) | O_NONBLOCK);
	clients[index].valid = 1;
	clients[index].fd_send = fd;
	valid_clients_num++;
	printf("New client entered, %d in total\n", valid_clients_num);
	return index;
}
ssize_t send_to_all(Client *client, const void *buf, size_t n)
{
	int size = 0;
	for_each_valid_client(other_client)
	{
		if (client != other_client)
		{
			int tmp_size = send(other_client->fd_send, buf, n, 0);
			printf("Client%ld sent %d bytes to client%ld starting with %d\n", client - clients, tmp_size, other_client - clients, *(char *)buf);
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
int main(int argc, char **argv)
{
	int port = atoi(argv[1]);
	int sockfd;
	if ((sockfd = socket(AF_INET, SOCK_STREAM | SOCK_NONBLOCK, 0)) == 0)
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
	if (listen(sockfd, 2))
	{
		perror("listen");
		return 1;
	}

	init_clients();

	fd_set clients_set;
	while (1)
	{
		int new_fd;
		if ((new_fd = accept(sockfd, NULL, NULL)) >= 0)
		{
			add_client(new_fd);
		}
		FD_ZERO(&clients_set);
		int max_fd = 0;
		for_each_valid_client(client)
		{
			FD_SET(client->fd_send, &clients_set);
			if (client->fd_send > max_fd)
				max_fd = client->fd_send;
		}
		struct timeval time_val = {0, 500};
		int ready_num = select(max_fd + 1, &clients_set, NULL, NULL, &time_val);
		if (ready_num == 0)
			continue;
		else if (ready_num > 0)
		{
			for_each_valid_client(client)
			{
				printf("Client%ld is to be tested\n", client - clients);
				if (FD_ISSET(client->fd_send, &clients_set))
				{
					printf("Client%ld is ready\n", client - clients);
					int is_during_contiguous_recv = 0;
					int is_during_contiguous_send = 0;
					ssize_t len;
					char recv_buffer[buffer_size + PADDING];
					char send_buffer[buffer_size + PADDING + NICK_NAME_MAX_LEN];
					sprintf(send_buffer, "From %s: ", client->nick_name);
					const int offset = strlen(send_buffer);
					const int send_buffer_size = offset + buffer_size;
					int recv_buffer_index = 0;
					int send_buffer_index = offset;
					do
					{
						printf("Clinet%ld is to recv\n", client - clients);
						recv_buffer_index = 0;
						memset(recv_buffer, 0, buffer_size + PADDING);
						len = receive(client, recv_buffer, buffer_size);
						if (!is_during_contiguous_recv && strcmp(recv_buffer, "$EXIT\n") == 0)
						{
							destroy_client(client);
							break;
						}
						if (len <= 0) // received nothing, terminate.
						{
							printf("Client%ld nothing received\n", client - clients);
							break;
						}
						while (1)
						{
							if (recv_buffer_index >= buffer_size)
							{
								printf("Client%ld: Massive data received\n", client - clients);
								is_during_contiguous_recv = 1;
								fcntl(client->fd_send, F_SETFL, fcntl(client->fd_send, F_GETFL, 0) & ~O_NONBLOCK);
								break; // Perform next reception
							}
							if (send_buffer_index >= send_buffer_size)
							{
								printf("Client%ld: Massive data sent\n", client - clients);
								send_to_all(client, send_start, send_len);
								is_during_contiguous_send = 1;
								send_buffer_index = offset;
							}
							if (is_com_char(recv_buffer[recv_buffer_index]))
							{
								send_buffer[send_buffer_index++] = recv_buffer[recv_buffer_index++];
							}
							else if (recv_buffer[recv_buffer_index] == '\n')
							{
								send_buffer[send_buffer_index++] = '\n';
								send_to_all(client, send_start, send_len);
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
								{
									printf("Client%ld received a messaged not ending with '\\n'\n", client - clients);
									send_to_all(client, send_start, send_len);
								}
								printf("Client%ld encounters EOF\n", client - clients);
								is_during_contiguous_recv = 0;
								break;
							}
						}
					} while (is_during_contiguous_recv);
					fcntl(client->fd_send, F_SETFL, fcntl(client->fd_send, F_GETFL, 0) | O_NONBLOCK);
				}
			}
		}
		else
			break;
	}
	return 0;
}