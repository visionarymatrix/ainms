package handler

import (
	"context"
	"log"

	"github.com/ainms/gateway/internal/service"
	"github.com/golang-jwt/jwt/v5"
	socketio "github.com/zishang520/socket.io/servers/socket/v3"
	"github.com/google/uuid"
	"github.com/zishang520/socket.io/v3/pkg/types"
)

type SocketHandler struct {
	hub             *service.SocketHub
	installTokenSvc *service.InstallTokenService
	authSvc         *service.AuthService
	enrollmentSvc    *service.EnrollmentService
	screenshotSvc    *service.ScreenshotService
}

func NewSocketHandler(
	hub *service.SocketHub,
	installTokenSvc *service.InstallTokenService,
	authSvc *service.AuthService,
	enrollmentSvc *service.EnrollmentService,
	screenshotSvc *service.ScreenshotService,
) *SocketHandler {
	return &SocketHandler{
		hub:             hub,
		installTokenSvc: installTokenSvc,
		authSvc:         authSvc,
		enrollmentSvc:    enrollmentSvc,
		screenshotSvc:    screenshotSvc,
	}
}

func (h *SocketHandler) RegisterEvents(server *socketio.Server) {
	server.On("connection", h.onConnection)
}

func (h *SocketHandler) onConnection(args ...any) {
	if len(args) == 0 {
		log.Println("[SocketHandler] connection event with no args")
		return
	}

	s, ok := args[0].(*socketio.Socket)
	if !ok {
		log.Println("[SocketHandler] connection event: first arg is not a *socket.Socket")
		return
	}

	query := s.Handshake().Query
	connType := getQueryParam(query, "type")

	switch connType {
	case "agent":
		h.handleAgentConnect(s)
	case "admin":
		h.handleAdminConnect(s)
	default:
		log.Printf("[SocketHandler] unknown connection type=%q, disconnecting socket %s", connType, s.Id())
		s.Disconnect(true)
	}
}

func (h *SocketHandler) handleAgentConnect(s *socketio.Socket) {
	token := getQueryParam(s.Handshake().Query, "token")
	if token == "" {
		log.Printf("[SocketHandler] agent connect rejected: missing token, socket=%s", s.Id())
		s.Disconnect(true)
		return
	}

	claims, err := h.installTokenSvc.ValidateInstallToken(token)
	if err != nil {
		log.Printf("[SocketHandler] agent connect rejected: invalid token, socket=%s err=%v", s.Id(), err)
		s.Disconnect(true)
		return
	}

	deviceID := getQueryParam(s.Handshake().Query, "device_id")
	if deviceID == "" {
		log.Printf("[SocketHandler] agent connect rejected: missing device_id, socket=%s", s.Id())
		s.Disconnect(true)
		return
	}

	if _, err := uuid.Parse(deviceID); err != nil {
		log.Printf("[SocketHandler] agent connect rejected: invalid device_id=%s, socket=%s", deviceID, s.Id())
		s.Disconnect(true)
		return
	}

	companyID := claims.CompanyID

	connInfo := &service.SocketConnInfo{
		Type:      "agent",
		DeviceID:  deviceID,
		CompanyID: companyID,
	}
	s.SetData(connInfo)

	s.Join(socketio.Room("device:" + deviceID))

	h.hub.RegisterAgent(deviceID, s)

	h.hub.MarkDeviceHeartbeat(context.Background(), deviceID, h.enrollmentSvc)

	h.hub.BroadcastToCompanyAdmins(companyID, "device_online", map[string]interface{}{
		"device_id":  deviceID,
		"company_id": companyID,
	})

	log.Printf("[SocketHandler] agent connected: device=%s company=%s socket=%s", deviceID, companyID, s.Id())

	s.On("screenshot_ready", h.handleScreenshotReady(s, connInfo))

	s.On("disconnect", func(args ...any) {
		log.Printf("[SocketHandler] agent disconnect: device=%s", deviceID)
		h.hub.UnregisterAgent(deviceID)
	})
}

func (h *SocketHandler) handleAdminConnect(s *socketio.Socket) {
	token := getQueryParam(s.Handshake().Query, "token")
	if token == "" {
		log.Printf("[SocketHandler] admin connect rejected: missing token, socket=%s", s.Id())
		s.Disconnect(true)
		return
	}

	parsed, err := jwt.Parse(token, func(t *jwt.Token) (interface{}, error) {
		return []byte("ainms-dev-secret-change-in-production"), nil
	})
	if err != nil || !parsed.Valid {
		log.Printf("[SocketHandler] admin connect rejected: invalid JWT, socket=%s err=%v", s.Id(), err)
		s.Disconnect(true)
		return
	}

	claims, ok := parsed.Claims.(jwt.MapClaims)
	if !ok {
		log.Printf("[SocketHandler] admin connect rejected: invalid claims, socket=%s", s.Id())
		s.Disconnect(true)
		return
	}

	userID, _ := claims["user_id"].(string)
	if userID == "" {
		log.Printf("[SocketHandler] admin connect rejected: missing user_id in claims, socket=%s", s.Id())
		s.Disconnect(true)
		return
	}

	var companyID string
	if cid, ok := claims["company_id"]; ok && cid != nil {
		if cidStr, ok := cid.(string); ok {
			companyID = cidStr
		}
	}

	if companyID == "" {
		log.Printf("[SocketHandler] admin connect rejected: missing company_id, socket=%s", s.Id())
		s.Disconnect(true)
		return
	}

	connInfo := &service.SocketConnInfo{
		Type:      "admin",
		UserID:    userID,
		CompanyID: companyID,
	}
	s.SetData(connInfo)

	s.Join(socketio.Room("company:" + companyID + ":admins"))

	h.hub.RegisterAdmin(userID, s)

	log.Printf("[SocketHandler] admin connected: user=%s company=%s socket=%s", userID, companyID, s.Id())

	s.On("screenshot_request", h.handleScreenshotRequest(s, connInfo))

	s.On("disconnect", func(args ...any) {
		log.Printf("[SocketHandler] admin disconnect: user=%s", userID)
		h.hub.UnregisterAdmin(userID)
	})
}

func (h *SocketHandler) handleScreenshotRequest(s *socketio.Socket, connInfo *service.SocketConnInfo) func(...any) {
	return func(args ...any) {
		if len(args) == 0 {
			log.Printf("[SocketHandler] screenshot_request: no payload from user=%s", connInfo.UserID)
			return
		}

		data, ok := args[0].(map[string]interface{})
		if !ok {
			log.Printf("[SocketHandler] screenshot_request: invalid payload type from user=%s", connInfo.UserID)
			return
		}

		deviceID, _ := data["device_id"].(string)
		if deviceID == "" {
			log.Printf("[SocketHandler] screenshot_request: missing device_id from user=%s", connInfo.UserID)
			return
		}

		reason := "On-demand screenshot via Socket.IO"
		if r, ok := data["reason"].(string); ok && r != "" {
			reason = r
		}
		policy := "upload_image"
		if p, ok := data["policy"].(string); ok && p != "" {
			policy = p
		}

		requestedBy, err := uuid.Parse(connInfo.UserID)
		if err != nil {
			log.Printf("[SocketHandler] screenshot_request: invalid user_id=%s err=%v", connInfo.UserID, err)
			return
		}

		deviceUUID, err := uuid.Parse(deviceID)
		if err != nil {
			log.Printf("[SocketHandler] screenshot_request: invalid device_id=%s err=%v", deviceID, err)
			return
		}

		log.Printf("[SocketHandler] screenshot_request: device=%s from user=%s", deviceID, connInfo.UserID)

		dbReq, dbErr := h.screenshotSvc.RequestScreenshot(context.Background(), deviceUUID, requestedBy, reason, policy)
		if dbErr != nil {
			log.Printf("[SocketHandler] failed to create screenshot request in DB: device=%s err=%v", deviceID, dbErr)
			return
		}

		err = h.hub.SendToAgent(deviceID, "screenshot_request", map[string]interface{}{
			"request_id":   dbReq.ID.String(),
			"device_id":    deviceID,
			"reason":       reason,
			"policy":       policy,
			"requested_by": connInfo.UserID,
		})
		if err != nil {
			log.Printf("[SocketHandler] failed to send screenshot_request to device=%s: %v", deviceID, err)
		}
	}
}

func (h *SocketHandler) handleScreenshotReady(s *socketio.Socket, connInfo *service.SocketConnInfo) func(...any) {
	return func(args ...any) {
		if len(args) == 0 {
			log.Printf("[SocketHandler] screenshot_ready: no payload from device=%s", connInfo.DeviceID)
			return
		}

		data, ok := args[0].(map[string]interface{})
		if !ok {
			log.Printf("[SocketHandler] screenshot_ready: invalid payload type from device=%s", connInfo.DeviceID)
			return
		}

		requestID, _ := data["request_id"].(string)
		if requestID == "" {
			log.Printf("[SocketHandler] screenshot_ready: missing request_id from device=%s", connInfo.DeviceID)
			return
		}

		if _, err := uuid.Parse(requestID); err != nil {
			log.Printf("[SocketHandler] screenshot_ready: invalid request_id=%s from device=%s", requestID, connInfo.DeviceID)
			return
		}

		log.Printf("[SocketHandler] screenshot_ready: request=%s from device=%s", requestID, connInfo.DeviceID)

		h.hub.BroadcastToCompanyAdmins(connInfo.CompanyID, "screenshot_ready", map[string]interface{}{
			"request_id": requestID,
			"device_id":  connInfo.DeviceID,
			"company_id": connInfo.CompanyID,
		})
	}
}

func getQueryParam(query types.ParsedUrlQuery, key string) string {
	q := query.Query()
	if vals, ok := q[key]; ok && len(vals) > 0 {
		return vals[0]
	}
	return ""
}

