package service

import (
	"context"
	"log"
	"sync"

	socketio "github.com/zishang520/socket.io/servers/socket/v3"
)

type SocketHub struct {
	sio    *socketio.Server
	agents map[string]*socketio.Socket
	admins map[string]*socketio.Socket
	mu     sync.RWMutex
}

func NewSocketHub(sio *socketio.Server) *SocketHub {
	return &SocketHub{
		sio:    sio,
		agents: make(map[string]*socketio.Socket),
		admins: make(map[string]*socketio.Socket),
	}
}

func (h *SocketHub) RegisterAgent(deviceID string, s *socketio.Socket) {
	h.mu.Lock()
	defer h.mu.Unlock()

	if old, ok := h.agents[deviceID]; ok {
		go old.Disconnect(true)
	}

	h.agents[deviceID] = s
	log.Printf("[SocketHub] agent registered: device=%s", deviceID)
}

func (h *SocketHub) RegisterAdmin(userID string, s *socketio.Socket) {
	h.mu.Lock()
	defer h.mu.Unlock()

	if old, ok := h.admins[userID]; ok {
		go old.Disconnect(true)
	}

	h.admins[userID] = s
	log.Printf("[SocketHub] admin registered: user=%s", userID)
}

func (h *SocketHub) UnregisterAgent(deviceID string) {
	h.mu.Lock()
	s, existed := h.agents[deviceID]
	if existed {
		delete(h.agents, deviceID)
	}
	companyID := ""
	if s != nil {
		if data := s.Data(); data != nil {
			if m, ok := data.(map[string]interface{}); ok {
				if cid, ok := m["company_id"].(string); ok {
					companyID = cid
				}
			}
		}
	}
	h.mu.Unlock()

	if existed {
		log.Printf("[SocketHub] agent unregistered: device=%s", deviceID)
		if companyID != "" {
			h.BroadcastToCompanyAdmins(companyID, "device_offline", map[string]interface{}{
				"device_id": deviceID,
			})
		}
	}
}

func (h *SocketHub) UnregisterAdmin(userID string) {
	h.mu.Lock()
	defer h.mu.Unlock()

	if _, existed := h.admins[userID]; existed {
		delete(h.admins, userID)
		log.Printf("[SocketHub] admin unregistered: user=%s", userID)
	}
}

func (h *SocketHub) SendToAgent(deviceID string, event string, data interface{}) error {
	h.mu.RLock()
	s, ok := h.agents[deviceID]
	h.mu.RUnlock()

	if !ok {
		log.Printf("[SocketHub] send to agent skipped: device=%s not connected", deviceID)
		return nil
	}

	if err := s.Emit(event, data); err != nil {
		log.Printf("[SocketHub] emit to agent failed: device=%s event=%s err=%v", deviceID, event, err)
		return err
	}

	log.Printf("[SocketHub] sent event to agent: device=%s event=%s", deviceID, event)
	return nil
}

func (h *SocketHub) BroadcastToCompanyAdmins(companyID string, event string, data interface{}) {
	room := "company:" + companyID + ":admins"
	h.sio.To(socketio.Room(room)).Emit(event, data)
	log.Printf("[SocketHub] broadcast to company admins: room=%s event=%s", room, event)
}

func (h *SocketHub) BroadcastToAll(event string, data interface{}) {
	h.sio.Emit(event, data)
}

func (h *SocketHub) GetOnlineDevices() []string {
	h.mu.RLock()
	defer h.mu.RUnlock()

	devices := make([]string, 0, len(h.agents))
	for id := range h.agents {
		devices = append(devices, id)
	}
	return devices
}

func (h *SocketHub) MarkDeviceHeartbeat(ctx context.Context, deviceID string, enrollmentSvc *EnrollmentService) {
	if err := enrollmentSvc.Heartbeat(ctx, deviceID, "", nil); err != nil {
		log.Printf("[SocketHub] heartbeat on connect failed: device=%s err=%v", deviceID, err)
	}
}

type SocketConnInfo struct {
	Type      string `json:"type"`
	DeviceID  string `json:"device_id"`
	UserID    string `json:"user_id"`
	CompanyID string `json:"company_id"`
}