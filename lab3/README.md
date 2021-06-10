# Lab03: Group Chat

## Implementation

* 处理不同步`recv`/`send`

  考虑到

  1. 接受数据应以`\n`为分隔符分段发送
  2. 接受数据可能大于buffer大小，需要分段接受
  3. 要发送的一条消息（即以`\n`划分成的一段数据）可能较大

  我分别设置了`recv_buffer`和`send_buffer`，用于存储单次接受的数据和单次发送的数据。

  对于二者的处理，可用以下伪代码描述：

  ```c
  while(recv(recv_buff)){
      for (i, c) in recv_buff.enumerate(){
          if(c == '\n'){
              send_buff[i] = '\n';
              send(send_buff);
          }else if(c == 0){
              send(send_buff);
          }else{
              send_buff[i] = c;
          }
      }
  }
  ```

  但需要注意两个情况：

  1. `recv_buff`已满，但未读到`\n`或EOF

     说明因`recv_buff`的大小限制，单次`recv`未能将所有消息读入，需要继续使用`recv`读入。

     此时，`send_buff`不变，程序重新回到循环起点接收，并将新数据接着写入`send_buff`.  

  2. `send_buff`已满，但未收到`\n`或EOF

     此时先将`send_buff`发送，下次发送的消息与此条是相连的，无需加上消息前缀，故需设置flag（本实现中使用`is_during_contiguous_send`）记录。

* 多线程处理群聊

  对每一个客户端，结构体`struct Client`存储了相关信息：

  ```c
  typedef struct Client
  {
  	int valid;              // 此结构体是否在使用中
  	int fd_send;            // 客户端对应发送文件描述符
  	pthread_mutex_t mutex;  // 互斥锁：对fd_send发送数据时需加锁
  	pthread_t thread;       // 运行此客户端的`handle_chat`的线程
  } Client;
  ```

  `Client[]`类型的`clients`存储了所有可能使用的`Client`。

  主线程无限、阻塞地从`sockfd`段`accept`；每当有新的请求被接受，程序`add_client`将使用`clients`中空闲位置放置新客户端的信息，并开启新线程。

  `handle_chat`的程序使用上一小段所述的不同步`recv/send`操作，唯一的区别在于发送时会遍历每一有效客户端，在加锁下对其发送消息。

* IO复用

  程序无限地进行`accept`与`select`操作：

  ```c
  while(1){
      int new_fd = accept(sockfd, NULL, NULL);
      if (new_fd >=0){
          add_client(new_fd);
      }
      ...
      int ready_num = select(max_fd + 1, &clients_set, NULL, NULL, &time_val);
      if(ready_num == 0) // 在timeout时间内未发现可用IO
          continue;
      else{
          /* handle chat*/
      }
  }
  ```

  为避免 a)因`accept`阻塞而导致无法到达`select` b)因`select`阻塞而无法加入新客户端，本实现中将`sockfd`设置为非阻塞（`socket(AF_INET, SOCK_STREAM | SOCK_NONBLOCK, 0))`），从而`accept`非阻塞运行；并使用`select`中`timeout`参数，使得一段时间内无可用IO后得以继续检查`accept`的结果。

  `handle_chat`部分与上一小段中不同的地方在于，其`recv`均为非阻塞，但对于因接收数据过大而为在单次`recv`接受完毕的情况，需要阻塞`recv`等待后续数据的到来。故当处在预期连续接受的状态时将使用阻塞IO。

