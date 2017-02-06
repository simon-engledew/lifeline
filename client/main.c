#include <arpa/inet.h>
#include <netdb.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
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

int connect_socket(const char* address, const char* port) {
    int sockfd = -1;
    struct addrinfo hints, *servinfo, *p;
    int rv;

    memset(&hints, 0, sizeof hints);
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;

    if ((rv = getaddrinfo(address, port, &hints, &servinfo)) != 0) {
        fprintf(stderr, "error: getaddrinfo %s\n", gai_strerror(rv));

        exit(EXIT_FAILURE);
    }

    for (p = servinfo; p != NULL; p = p->ai_next) {
        if ((sockfd = socket(p->ai_family, p->ai_socktype, p->ai_protocol)) == -1) {
            continue;
        }

        if (connect(sockfd, p->ai_addr, p->ai_addrlen) == -1) {
            close(sockfd);
            continue;
        }

        break;
    }

    freeaddrinfo(servinfo);

    return sockfd;
}

int main(int argc, char* argv[])
{
    if (argc != 2 && argc != 3)
    {
        printf("Usage: %s [TARGET] <PORT>\n", argv[0]);
        return EXIT_FAILURE;
    }

    const char* port = argc == 3 ? argv[2] : "8888";

    int sockfd = -1;

    if ((sockfd = connect_socket(argv[1], port)) < 0)
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
