package config

import (
	"fmt"
	"os"
	"strconv"
)

// Config holds all configuration for the API gateway.
type Config struct {
	Server     ServerConfig
	Postgres   PostgresConfig
	ClickHouse ClickHouseConfig
	Redis      RedisConfig
	MinIO      MinIOConfig
	Ollama     OllamaConfig
	UploadDir  string
}

// ServerConfig holds HTTP server configuration.
type ServerConfig struct {
	Host string
	Port int
}

// PostgresConfig holds PostgreSQL connection configuration.
type PostgresConfig struct {
	Host     string
	Port     int
	User     string
	Password string
	DB       string
	SSLMode  string
}

// ClickHouseConfig holds ClickHouse connection configuration.
type ClickHouseConfig struct {
	Host     string
	Port     int
	User     string
	Password string
	DB       string
}

// RedisConfig holds Redis connection configuration.
type RedisConfig struct {
	Host     string
	Port     int
	Password string
	DB       int
}

// MinIOConfig holds MinIO configuration.
type MinIOConfig struct {
	Endpoint  string
	AccessKey string
	SecretKey string
	Bucket    string
	Secure    bool
}

// OllamaConfig holds Ollama Cloud API configuration.
type OllamaConfig struct {
	BaseURL string
	APIKey  string
	Model   string
	Timeout int
}

// Load returns a Config populated from environment variables with sensible defaults.
func Load() (*Config, error) {
	cfg := &Config{
		Server: ServerConfig{
			Host: getEnv("SERVER_HOST", "0.0.0.0"),
			Port: getEnvInt("SERVER_PORT", 8440),
		},
		Postgres: PostgresConfig{
			Host:     getEnv("POSTGRES_HOST", "localhost"),
			Port:     getEnvInt("POSTGRES_PORT", 5440),
			User:     getEnv("POSTGRES_USER", "ainms"),
			Password: getEnv("POSTGRES_PASSWORD", "ainms_dev_password"),
			DB:       getEnv("POSTGRES_DB", "ainms"),
			SSLMode:  getEnv("POSTGRES_SSL_MODE", "disable"),
		},
		ClickHouse: ClickHouseConfig{
			Host:     getEnv("CLICKHOUSE_HOST", "localhost"),
			Port:     getEnvInt("CLICKHOUSE_PORT", 8123),
			User:     getEnv("CLICKHOUSE_USER", "default"),
			Password: getEnv("CLICKHOUSE_PASSWORD", ""),
			DB:       getEnv("CLICKHOUSE_DB", "ainms"),
		},
		Redis: RedisConfig{
			Host:     getEnv("REDIS_HOST", "localhost"),
			Port:     getEnvInt("REDIS_PORT", 6390),
			Password: getEnv("REDIS_PASSWORD", ""),
			DB:       getEnvInt("REDIS_DB", 0),
		},
		MinIO: MinIOConfig{
			Endpoint:  getEnv("MINIO_ENDPOINT", "localhost:9101"),
			AccessKey: getEnv("MINIO_ACCESS_KEY", "ainms_minio"),
			SecretKey: getEnv("MINIO_SECRET_KEY", "ainms_minio_password"),
			Bucket:    getEnv("MINIO_BUCKET", "ainms-screenshots"),
			Secure:    getEnvBool("MINIO_SECURE", false),
		},
		Ollama: OllamaConfig{
			BaseURL: getEnv("OLLAMA_BASE_URL", "https://ollama.com/api"),
			APIKey:  getEnv("OLLAMA_API_KEY", ""),
			Model:   getEnv("OLLAMA_MODEL", "gemma4:31b-cloud"),
			Timeout: getEnvInt("OLLAMA_TIMEOUT", 60),
		},
		UploadDir: getEnv("UPLOAD_DIR", "public/screenshots"),
	}

	return cfg, nil
}

// PostgresDSN returns the PostgreSQL connection string.
func (c *PostgresConfig) DSN() string {
	return fmt.Sprintf(
		"postgres://%s:%s@%s:%d/%s?sslmode=%s",
		c.User, c.Password, c.Host, c.Port, c.DB, c.SSLMode,
	)
}

// ClickHouseDSN returns the ClickHouse connection string.
func (c *ClickHouseConfig) DSN() string {
	return fmt.Sprintf("http://%s:%d?database=%s&username=%s&password=%s",
		c.Host, c.Port, c.DB, c.User, c.Password,
	)
}

// RedisAddr returns the Redis address string.
func (c *RedisConfig) Addr() string {
	return fmt.Sprintf("%s:%d", c.Host, c.Port)
}

func getEnv(key, fallback string) string {
	if val, ok := os.LookupEnv(key); ok {
		return val
	}
	return fallback
}

func getEnvInt(key string, fallback int) int {
	if val, ok := os.LookupEnv(key); ok {
		if i, err := strconv.Atoi(val); err == nil {
			return i
		}
	}
	return fallback
}

func getEnvBool(key string, fallback bool) bool {
	if val, ok := os.LookupEnv(key); ok {
		if b, err := strconv.ParseBool(val); err == nil {
			return b
		}
	}
	return fallback
}