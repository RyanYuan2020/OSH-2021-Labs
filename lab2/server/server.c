
#include <sys/types.h>
#include <sys/socket.h>
#include <stdio.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <unistd.h>
#include <string.h>
#include <stdlib.h>
#include <fcntl.h>
#include <sys/shm.h>
#include <sys/wait.h>

#define RECVPORT 8887
#define SENDPORT 8888
#define QUEUE 20
#define BUFFER_SIZE 1024

int main()
{
	int recv_server_pid = fork();
	if (recv_server_pid < 0)
	{
		perror("Failed to fork recv server");
	}
	else if (recv_server_pid == 0) // Child process to infinitely receive message
	{
		int server_sockfd = socket(AF_INET, SOCK_STREAM, 0);

		struct sockaddr_in server_sockaddr;
		server_sockaddr.sin_family = AF_INET;
		server_sockaddr.sin_port = htons(RECVPORT);
		server_sockaddr.sin_addr.s_addr = htonl(INADDR_ANY);

		if (bind(server_sockfd, (struct sockaddr *)&server_sockaddr, sizeof(server_sockaddr)) == -1)
		{
			perror("Failed to bind");
			exit(1);
		}

		printf("Listen port %d\n", RECVPORT);
		if (listen(server_sockfd, QUEUE) == -1)
		{
			perror("Failed while listening");
			exit(1);
		}

		char buffer[BUFFER_SIZE];
		struct sockaddr_in client_addr;
		socklen_t length = sizeof(client_addr);

		while (1)
		{
			printf("Waiting for client connection\n");

			int conn = accept(server_sockfd, (struct sockaddr *)&client_addr, &length);
			if (conn < 0)
			{
				perror("Failed to connect");
				exit(1);
			}
			printf("Client is successfully connected\n");
			while (1)
			{
				memset(buffer, 0, sizeof(buffer));
				int len = recv(conn, buffer, sizeof(buffer), 0);
				if (strcmp(buffer, "exit\n") == 0 || len <= 0)
				{
					close(conn);
					break;
				}
				printf("Data received from client：%s\n", buffer);
			}
		}
		close(server_sockfd);
	}
	else
	{
		int send_server_pid = fork();
		if (send_server_pid < 0)
		{
			perror("Failed to fork send server");
		}
		else if (send_server_pid == 0) // Child process to infinitely sent message
		{
			int server_sockfd = socket(AF_INET, SOCK_STREAM, 0);

			struct sockaddr_in server_sockaddr;
			server_sockaddr.sin_family = AF_INET;
			server_sockaddr.sin_port = htons(SENDPORT);
			server_sockaddr.sin_addr.s_addr = htonl(INADDR_ANY);

			if (bind(server_sockfd, (struct sockaddr *)&server_sockaddr, sizeof(server_sockaddr)) == -1)
			{
				perror("Failed to bind at send server");
				exit(1);
			}

			printf("Listen port %d\n", SENDPORT);
			if (listen(server_sockfd, QUEUE) == -1)
			{
				perror("Failed while listening");
				exit(1);
			}

			char buffer[BUFFER_SIZE];
			struct sockaddr_in client_addr;
			socklen_t length = sizeof(client_addr);

			while (1)
			{
				printf("Waiting for client connection\n");

				int conn = accept(server_sockfd, (struct sockaddr *)&client_addr, &length);
				if (conn < 0)
				{
					perror("Failed to connect");
					exit(1);
				}
				printf("Client is successfully connected\n");

				const char *msg = "Hello World!\nLine2\nLine3\n";
				send(conn, msg, strlen(msg), 0);
				printf("Data sent to client：%s\n", msg);
				close(conn);
			}
			close(server_sockfd);
		}
		else // Parent process
		{
			while (wait(NULL) != -1)
				;
		}
	}
	return 0;
}