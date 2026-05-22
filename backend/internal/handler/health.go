package handler

import (
	"net/http"

	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/redis/go-redis/v9"
)

func Health(pg *pgxpool.Pool, ch interface{}, rdb *redis.Client) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if err := pg.Ping(r.Context()); err != nil {
			writeJSON(w, http.StatusServiceUnavailable, map[string]string{"status": "unhealthy", "postgres": err.Error()})
			return
		}
		writeJSON(w, http.StatusOK, map[string]string{"status": "healthy"})
	}
}