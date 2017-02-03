#include <arpa/inet.h>
#include <netdb.h>
// #include <netinet/in.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
// #include <sys/socket.h>
#include <sys/stat.h>
// #include <sys/types.h>
#include <unistd.h>

pid_t daemonize() {
    FILE *fp= NULL;

    pid_t process_id = 0;
    pid_t sid = 0;

    process_id = fork();

    if (process_id < 0)
    {
        printf("fork failed!\n");

        exit(EXIT_FAILURE);
    }

    if (process_id > 0)
    {
        return process_id;
    }

    umask(0);

    sid = setsid();

    if (sid < 0)
    {
        exit(EXIT_FAILURE);
    }

    close(STDIN_FILENO);
    close(STDOUT_FILENO);
    close(STDERR_FILENO);

    return process_id;
}

int main(int argc, char* argv[])
{
    if (argc != 2 && argc != 3)
    {
        printf("Usage: %s [TARGET] <PORT>\n", argv[0]);
        return EXIT_FAILURE;
    }

    unsigned short port = 8888;

    if (argc == 3) {
        port = atoi(argv[2]);
    }

    if (port == 0) {
        fprintf(stderr, "error: invalid port\n");
        exit(EXIT_FAILURE);
    }

    char protoname[] = "tcp";
    struct protoent *protoent = getprotobyname(protoname);
    if (protoent == NULL) {
        fprintf(stderr, "error: getprotobyname\n");
        exit(EXIT_FAILURE);
    }

    int sockfd = socket(AF_INET, SOCK_STREAM, protoent->p_proto);
    if (sockfd == -1) {
        fprintf(stderr, "error: could not create socket\n");
        exit(EXIT_FAILURE);
    }

    struct sockaddr_in serv_addr;

    memset(&serv_addr, '0', sizeof(serv_addr));
    serv_addr.sin_family = AF_INET;
    serv_addr.sin_port = htons(port);

    struct hostent *hostent = gethostbyname(argv[1]);
    if (hostent == NULL) {
        fprintf(stderr, "error: gethostbyname(\"%s\")\n", argv[1]);

        exit(EXIT_FAILURE);
    }

    in_addr_t in_addr = inet_addr(inet_ntoa(*(struct in_addr*)*(hostent->h_addr_list)));
    if (in_addr == (in_addr_t)-1) {
        fprintf(stderr, "error: inet_addr(\"%s\")\n", *(hostent->h_addr_list));
        exit(EXIT_FAILURE);
    }

    serv_addr.sin_addr.s_addr = in_addr;

    if (connect(sockfd, (struct sockaddr *)&serv_addr, sizeof(serv_addr)) < 0)
    {
       fprintf(stderr, "error: connect failed\n");
       return EXIT_FAILURE;
    }

    pid_t process_id = daemonize();

    if (process_id > 0) {
        printf("%d\n", process_id);

        close(sockfd);

        exit(EXIT_SUCCESS);
    }

    pause();

    return (EXIT_SUCCESS);
}
