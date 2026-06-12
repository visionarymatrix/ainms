package main

import (
	"context"
	"fmt"
	"os"

	"github.com/ainms/gateway/internal/config"
	"github.com/ainms/gateway/internal/repository/postgres"
	"github.com/jackc/pgx/v5"
)

func main() {
	cfg, err := config.Load()
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to load config: %v\n", err)
		os.Exit(1)
	}

	ctx := context.Background()
	pool, err := postgres.NewPool(ctx, cfg.Postgres.DSN())
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to connect to PostgreSQL: %v\n", err)
		os.Exit(1)
	}
	defer pool.Close()

	migrationSQL := `
	CREATE TABLE IF NOT EXISTS installed_apps (
		id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
		device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
		app_name TEXT NOT NULL,
		display_name TEXT NOT NULL DEFAULT '',
		publisher TEXT NOT NULL DEFAULT '',
		install_path TEXT,
		category TEXT NOT NULL DEFAULT 'neutral',
		confidence DOUBLE PRECISION NOT NULL DEFAULT 0.0,
		source TEXT NOT NULL DEFAULT 'unknown',
		created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
		updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
		UNIQUE(device_id, app_name)
	);

	CREATE INDEX IF NOT EXISTS idx_installed_apps_device_id ON installed_apps(device_id);
	CREATE INDEX IF NOT EXISTS idx_installed_apps_app_name ON installed_apps(app_name);
	`

	_, err = pool.Exec(ctx, migrationSQL)
	if err != nil {
		// Try without foreign key if devices table doesn't exist (test mode)
		simpleSQL := `
		CREATE TABLE IF NOT EXISTS installed_apps (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			device_id UUID NOT NULL,
			app_name TEXT NOT NULL,
			display_name TEXT NOT NULL DEFAULT '',
			publisher TEXT NOT NULL DEFAULT '',
			install_path TEXT,
			category TEXT NOT NULL DEFAULT 'neutral',
			confidence DOUBLE PRECISION NOT NULL DEFAULT 0.0,
			source TEXT NOT NULL DEFAULT 'unknown',
			created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
			updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
			UNIQUE(device_id, app_name)
		);

		CREATE INDEX IF NOT EXISTS idx_installed_apps_device_id ON installed_apps(device_id);
		CREATE INDEX IF NOT EXISTS idx_installed_apps_app_name ON installed_apps(app_name);
		`
		_, err = pool.Exec(ctx, simpleSQL)
		if err != nil {
			fmt.Fprintf(os.Stderr, "failed to apply migration: %v\n", err)
			os.Exit(1)
		}
	}

	// Check if the column exists, if not add it
	var columnName string
	err = pool.QueryRow(ctx, `
		SELECT column_name FROM information_schema.columns 
		WHERE table_name = 'installed_apps' AND column_name = 'install_path'
	`).Scan(&columnName)

	if err == pgx.ErrNoRows {
		// Column doesn't exist, add it
		_, err = pool.Exec(ctx, `ALTER TABLE installed_apps ADD COLUMN IF NOT EXISTS install_path TEXT`)
		if err != nil {
			fmt.Fprintf(os.Stderr, "failed to add install_path column: %v\n", err)
			os.Exit(1)
		}
	}

	fmt.Println("Migration 022 applied successfully: installed_apps table created")
}