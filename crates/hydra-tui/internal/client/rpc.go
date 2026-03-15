package client

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"sync/atomic"
	"time"
)

const defaultBaseURL = "http://127.0.0.1:7777"

var requestID uint64

// HydraRpcClient connects to hydra-server via JSON-RPC + REST.
type HydraRpcClient struct {
	BaseURL string
	client  *http.Client
}

// NewRpcClient creates a new client.
func NewRpcClient() *HydraRpcClient {
	baseURL := os.Getenv("HYDRA_SERVER_URL")
	if baseURL == "" {
		baseURL = defaultBaseURL
	}
	return &HydraRpcClient{
		BaseURL: baseURL,
		client:  &http.Client{Timeout: 30 * time.Second},
	}
}

func (c *HydraRpcClient) rpcCall(method string, params interface{}) (json.RawMessage, error) {
	id := atomic.AddUint64(&requestID, 1)
	req := RpcRequest{
		Jsonrpc: "2.0",
		Method:  method,
		Params:  params,
		ID:      id,
	}
	body, err := json.Marshal(req)
	if err != nil {
		return nil, fmt.Errorf("marshal request: %w", err)
	}

	resp, err := c.client.Post(c.BaseURL+"/rpc", "application/json", bytes.NewReader(body))
	if err != nil {
		return nil, fmt.Errorf("rpc call failed: %w", err)
	}
	defer resp.Body.Close()

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("read response: %w", err)
	}

	var rpcResp RpcResponse
	if err := json.Unmarshal(data, &rpcResp); err != nil {
		return nil, fmt.Errorf("parse response: %w", err)
	}
	if rpcResp.Error != nil {
		return nil, fmt.Errorf("rpc error %d: %s", rpcResp.Error.Code, rpcResp.Error.Message)
	}

	raw, err := json.Marshal(rpcResp.Result)
	if err != nil {
		return nil, fmt.Errorf("marshal result: %w", err)
	}
	return raw, nil
}

func (c *HydraRpcClient) get(path string) (json.RawMessage, error) {
	resp, err := c.client.Get(c.BaseURL + path)
	if err != nil {
		return nil, fmt.Errorf("GET failed: %w", err)
	}
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}
	return data, nil
}

// Health calls hydra.health.
func (c *HydraRpcClient) Health() (*HealthInfo, error) {
	raw, err := c.rpcCall("hydra.health", map[string]interface{}{})
	if err != nil {
		return nil, err
	}
	var info HealthInfo
	if err := json.Unmarshal(raw, &info); err != nil {
		return nil, err
	}
	return &info, nil
}

// Run calls hydra.run.
func (c *HydraRpcClient) Run(input string) (*RunResult, error) {
	raw, err := c.rpcCall("hydra.run", map[string]string{"input": input})
	if err != nil {
		return nil, err
	}
	var result RunResult
	if err := json.Unmarshal(raw, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Cancel calls hydra.cancel.
func (c *HydraRpcClient) Cancel(runID string) error {
	_, err := c.rpcCall("hydra.cancel", map[string]string{"run_id": runID})
	return err
}

// Approve calls hydra.approve.
func (c *HydraRpcClient) Approve(runID, decision string) error {
	_, err := c.rpcCall("hydra.approve", map[string]string{
		"run_id": runID, "decision": decision,
	})
	return err
}

// ProfileList calls hydra.profile.list.
func (c *HydraRpcClient) ProfileList() ([]ProfileInfo, error) {
	raw, err := c.rpcCall("hydra.profile.list", map[string]interface{}{})
	if err != nil {
		return nil, err
	}
	var profiles []ProfileInfo
	if err := json.Unmarshal(raw, &profiles); err != nil {
		return nil, err
	}
	return profiles, nil
}

// ProfileLoad calls hydra.profile.load.
func (c *HydraRpcClient) ProfileLoad(name string) error {
	_, err := c.rpcCall("hydra.profile.load", map[string]string{"name": name})
	return err
}

// ProfileUnload calls hydra.profile.unload.
func (c *HydraRpcClient) ProfileUnload() error {
	_, err := c.rpcCall("hydra.profile.unload", map[string]interface{}{})
	return err
}

// ROI calls hydra.roi.
func (c *HydraRpcClient) ROI() (*RoiSummary, error) {
	raw, err := c.rpcCall("hydra.roi", map[string]interface{}{})
	if err != nil {
		return nil, err
	}
	var roi RoiSummary
	if err := json.Unmarshal(raw, &roi); err != nil {
		return nil, err
	}
	return &roi, nil
}

// Status calls hydra.status.
func (c *HydraRpcClient) Status() (json.RawMessage, error) {
	return c.rpcCall("hydra.status", map[string]interface{}{})
}

// HealthCheck returns true if server is reachable.
func (c *HydraRpcClient) HealthCheck() bool {
	_, err := c.get("/health")
	if err == nil {
		return true
	}
	_, err = c.get("/api/system/status")
	return err == nil
}
