package client

import (
	"bufio"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
)

// StreamReceiver subscribes to SSE and sends chunks through a channel.
type StreamReceiver struct {
	Ch      chan StreamChunk
	baseURL string
	done    chan struct{}
}

// NewStreamReceiver creates and starts an SSE listener.
func NewStreamReceiver(baseURL string) *StreamReceiver {
	sr := &StreamReceiver{
		Ch:      make(chan StreamChunk, 100),
		baseURL: baseURL,
		done:    make(chan struct{}),
	}
	go sr.listen()
	return sr
}

func (sr *StreamReceiver) listen() {
	defer close(sr.Ch)

	req, err := http.NewRequest("GET", sr.baseURL+"/events", nil)
	if err != nil {
		fmt.Fprintf(nil, "[hydra-tui] SSE request error: %v\n", err)
		return
	}
	req.Header.Set("Accept", "text/event-stream")

	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		return // server not available
	}
	defer resp.Body.Close()

	scanner := bufio.NewScanner(resp.Body)
	var currentEvent, currentData string

	for scanner.Scan() {
		select {
		case <-sr.done:
			return
		default:
		}

		line := scanner.Text()

		if line == "" {
			// Event boundary
			if currentData != "" {
				var chunk StreamChunk
				if err := json.Unmarshal([]byte(currentData), &chunk); err == nil {
					sr.Ch <- chunk
				}
			}
			currentEvent = ""
			currentData = ""
		} else if strings.HasPrefix(line, "event: ") {
			currentEvent = line[7:]
			_ = currentEvent // suppress unused warning
		} else if strings.HasPrefix(line, "data: ") {
			if currentData != "" {
				currentData += "\n"
			}
			currentData += line[6:]
		}
	}
}

// Stop closes the stream.
func (sr *StreamReceiver) Stop() {
	close(sr.done)
}

// Drain returns all available chunks without blocking.
func (sr *StreamReceiver) Drain() []StreamChunk {
	var chunks []StreamChunk
	for {
		select {
		case chunk, ok := <-sr.Ch:
			if !ok {
				return chunks
			}
			chunks = append(chunks, chunk)
		default:
			return chunks
		}
	}
}
