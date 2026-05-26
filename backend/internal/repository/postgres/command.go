package postgres

import (
	"context"
	"fmt"

	"github.com/ainms/gateway/internal/domain"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

type CommandRepo struct {
	pool *pgxpool.Pool
}

func NewCommandRepo(pool *pgxpool.Pool) *CommandRepo {
	return &CommandRepo{pool: pool}
}

const commandColumns = `id, device_id, command_type, payload, status, created_at, sent_at, acked_at`

func scanCommand(scanner interface{ Scan(...interface{}) error }, c *domain.PendingCommandDB) error {
	return scanner.Scan(
		&c.ID, &c.DeviceID, &c.CommandType, &c.Payload, &c.Status,
		&c.CreatedAt, &c.SentAt, &c.AckedAt,
	)
}

func (r *CommandRepo) Create(ctx context.Context, cmd *domain.PendingCommandDB) error {
	query := `INSERT INTO pending_commands (id, device_id, command_type, payload, status, created_at)
		VALUES ($1, $2, $3, $4, $5, NOW())
		RETURNING created_at`

	if cmd.ID == uuid.Nil {
		cmd.ID = uuid.New()
	}

	return r.pool.QueryRow(ctx, query,
		cmd.ID, cmd.DeviceID, cmd.CommandType, cmd.Payload, cmd.Status,
	).Scan(&cmd.CreatedAt)
}

func (r *CommandRepo) ListPendingByDevice(ctx context.Context, deviceID uuid.UUID) ([]domain.PendingCommandDB, error) {
	query := `SELECT ` + commandColumns + ` FROM pending_commands WHERE device_id = $1 AND status IN ('pending', 'sent') ORDER BY created_at ASC`

	rows, err := r.pool.Query(ctx, query, deviceID)
	if err != nil {
		return nil, fmt.Errorf("list pending commands: %w", err)
	}
	defer rows.Close()

	commands := make([]domain.PendingCommandDB, 0)
	for rows.Next() {
		var c domain.PendingCommandDB
		if err := scanCommand(rows, &c); err != nil {
			return nil, fmt.Errorf("scan command: %w", err)
		}
		commands = append(commands, c)
	}
	return commands, rows.Err()
}

func (r *CommandRepo) MarkSent(ctx context.Context, id uuid.UUID) error {
	query := `UPDATE pending_commands SET status = 'sent', sent_at = NOW() WHERE id = $1`
	res, err := r.pool.Exec(ctx, query, id)
	if err != nil {
		return fmt.Errorf("mark command sent: %w", err)
	}
	if res.RowsAffected() == 0 {
		return fmt.Errorf("command not found: %s", id)
	}
	return nil
}

func (r *CommandRepo) MarkAcked(ctx context.Context, id uuid.UUID) error {
	query := `UPDATE pending_commands SET status = 'acked', acked_at = NOW() WHERE id = $1`
	res, err := r.pool.Exec(ctx, query, id)
	if err != nil {
		return fmt.Errorf("mark command acked: %w", err)
	}
	if res.RowsAffected() == 0 {
		return fmt.Errorf("command not found: %s", id)
	}
	return nil
}