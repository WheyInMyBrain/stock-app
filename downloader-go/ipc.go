// stock-app/downloader/ipc.go

package main

import (
	"encoding/binary"
	"fmt"
	"net"
	"os"
	"path/filepath"
)

// IPCServer wraps our local Unix Domain Socket listener engine context
type IPCServer struct {
	listener net.Listener
	socketPath string
}

// NewIPCServer initializes the socket file structure cleanly inside your active workspace path
func NewIPCServer(dataDir string) (*IPCServer, error) {
	// Anchoring the socket file cleanly inside your current app tracking directory
	socketPath := filepath.Join(dataDir, "downloader_engine.sock")

	// If a stale socket file was left behind from a previous hard crash, wipe it out completely
	_ = os.Remove(socketPath)

	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		return nil, fmt.Errorf("failed binding local domain socket link: %w", err)
	}

	return &IPCServer{
		listener:   listener,
		socketPath: socketPath,
	}, nil
}

// Close ensures the socket file is cleaned up off the SSD when Tauri exits
func (s *IPCServer) Close() {
	if s.listener != nil {
		_ = s.listener.Close()
	}
	_ = os.Remove(s.socketPath)
}

// WriteFramedPayload transmits data using a standard Length-Prefixed Binary Frame.
// This allows Rust to read the data instantly out of the network buffer without string parsing!
func WriteFramedPayload(conn net.Conn, jsonString string) error {
	payloadBytes := []byte(jsonString)
	payloadLength := uint32(len(payloadBytes))

	// 1. Allocate a 4-byte header buffer array
	headerBuffer := make([]byte, 4)
	
	// 2. Encode the length as a Big-Endian 32-bit integer inside the header
	binary.BigEndian.PutUint32(headerBuffer, payloadLength)

	// 3. Blast the 4-byte header over the socket line
	_, err := conn.Write(headerBuffer)
	if err != nil {
		return fmt.Errorf("failed transmitting binary frame header: %w", err)
	}

	// 4. Blast the actual raw JSON bytes payload immediately behind it
	_, err = conn.Write(payloadBytes)
	if err != nil {
		return fmt.Errorf("failed transmitting binary frame payload data: %w", err)
	}

	return nil
}