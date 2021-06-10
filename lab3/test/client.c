// Client side C/C++ program to demonstrate Socket programming
#include <stdio.h>
#include <sys/socket.h>
#include <arpa/inet.h>
#include <unistd.h>
#include <string.h>
#include <stdlib.h>
#include <pthread.h>
int sock;
void *recv_handler(void *args)
{
	FILE *output = fopen("recv.txt", "w+");
	char buffer[1 << 30] = {0};
	read(sock, buffer, 1 << 30);
	fprintf(output, "%s", buffer);
	return NULL;
}

int main(int argc, char const *argv[])
{
	int port = atoi(argv[1]);
	struct sockaddr_in serv_addr;
	char buffer[1 << 21] = {0};
	if ((sock = socket(AF_INET, SOCK_STREAM, 0)) < 0)
	{
		printf("\n Socket creation error \n");
		return -1;
	}

	serv_addr.sin_family = AF_INET;
	serv_addr.sin_port = htons(port);

	// Convert IPv4 and IPv6 addresses from text to binary form
	if (inet_pton(AF_INET, "127.0.0.1", &serv_addr.sin_addr) <= 0)
	{
		printf("\nInvalid address/ Address not supported \n");
		return -1;
	}

	if (connect(sock, (struct sockaddr *)&serv_addr, sizeof(serv_addr)) < 0)
	{
		printf("\nConnection Failed \n");
		return -1;
	}
	int integer;
	scanf("%d", &integer);
	pthread_t recv_thread;
	pthread_create(&recv_thread, NULL, recv_handler, NULL);
	FILE *input = fopen("test.txt", "r+");
	fscanf(input, "%s", buffer);
	strcat(buffer, "\n");
	send(sock, buffer, strlen(buffer), 0);
	pthread_join(recv_thread, NULL);
	return 0;
}
