package main

import (
	"fmt"
	"log"
	"net"
	"os"
	"os/exec"
	"time"

	"gopkg.in/alecthomas/kingpin.v2"
)

var (
	app     = kingpin.New(os.Args[0], "Run a command while clients are tethered")
	port    = app.Flag("port", "port to serve on").Default("8888").Uint32()
	grace   = app.Flag("grace-period", "Time to wait for initial peers").Default("5s").Duration()
	command = app.Arg("command", "command to run while peers are tethered").Required().String()
)

func main() {
	kingpin.MustParse(app.Parse(os.Args[1:]))

	bind := fmt.Sprintf("0.0.0.0:%d", *port)

	log.Println("Binding to", bind)

	listener, err := net.Listen("tcp", bind)
	if err != nil {
		return
	}

	log.Println("Starting", *command, "with", *grace, "grace")

	args := []string{"-c"}

	args = append(args, *command)

	command := exec.Command("/bin/sh", args...)
	command.Stdout = os.Stdout
	command.Stderr = os.Stderr
	err = command.Start()

	if err != nil {
		panic(err)
	}
	go func() {
		command.Wait()
		listener.Close()
	}()

	defer command.Process.Kill()
	defer listener.Close()

	connected := make(chan int)

	go func() {
		timeout := make(chan bool, 1)

		go func() {
			time.Sleep(*grace)
			timeout <- true
		}()
		current := 0
		for {
			select {
			case count := <-connected:
				current += count
			case <-timeout:
				if current == 0 {
					log.Println("Shutting down due to inactivity")
					break
				}
			}
			if current == 0 {
				log.Println("Last client has disconnected")
				break
			}
		}
		listener.Close()
	}()

	for {
		client, err := listener.Accept()
		if err != nil {
			return
		}

		go handleRequest(client, connected)
	}
}

func handleRequest(conn net.Conn, connected chan int) {
	connected <- 1
	log.Println("Client connected from", conn.RemoteAddr())

	defer func() {
		log.Println("Client disconnected from", conn.RemoteAddr())
		connected <- -1
	}()

	buffer := make([]byte, 4096)
	for {
		read, err := conn.Read(buffer)
		if err != nil || read == 0 {
			return
		}
	}
}
