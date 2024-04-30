package main

// #include <stdlib.h>
// #include <stdio.h>
//
// void println_from_c(char* s) {
//   printf("%s\n", s);
// }
//
import "C"
import (
	"log"
	"net"
	"net/http"
	"net/url"
	"unsafe"
)

func main() {
	ln, err := net.Listen("tcp", ":0")
	if err != nil {
		log.Fatalf("failed to listen: %w", err)
	}
	if err != nil {
		log.Fatalf("failed to create server: %v", err)
	}
	go http.Serve(ln, &server{})

	for {
		queryUrl := url.URL{
			Scheme: "http",
			Host:   ln.Addr().String(),
			Path:   "/",
		}
		http.Get(queryUrl.String())
	}
}

type server struct{}

func (s *server) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	cStr := C.CString("Hello from C")
	defer C.free(unsafe.Pointer(cStr))
	C.println_from_c(cStr)
}
