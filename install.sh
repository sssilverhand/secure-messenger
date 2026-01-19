#!/bin/bash
#
# PrivMsg Auto-Installer
# Автоматическая установка на Ubuntu/Debian
#

set -e

# Цвета для вывода
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}"
echo "╔═══════════════════════════════════════════╗"
echo "║       PrivMsg Server Installer            ║"
echo "║     Private E2EE Messenger Server         ║"
echo "╚═══════════════════════════════════════════╝"
echo -e "${NC}"

# Проверка root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Ошибка: Запустите скрипт от root (sudo)${NC}"
    exit 1
fi

# Определяем ОС
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
else
    echo -e "${RED}Ошибка: Не удалось определить ОС${NC}"
    exit 1
fi

echo -e "${GREEN}[✓]${NC} ОС: $OS $VERSION_ID"

# Генерация случайных ключей
generate_key() {
    tr -dc 'A-Za-z0-9' < /dev/urandom | head -c 32
}

ADMIN_KEY=$(generate_key)
TURN_PASSWORD=$(generate_key)

echo ""
echo -e "${YELLOW}Сгенерированы ключи:${NC}"
echo -e "  Admin Key: ${GREEN}$ADMIN_KEY${NC}"
echo -e "  TURN Password: ${GREEN}$TURN_PASSWORD${NC}"
echo ""
echo -e "${YELLOW}ВАЖНО: Сохраните Admin Key! Он понадобится для управления сервером.${NC}"
echo ""

read -p "Нажмите Enter для продолжения..."

# Обновление системы
echo ""
echo -e "${BLUE}[1/6]${NC} Обновление системы..."
apt update && apt upgrade -y

# Установка зависимостей
echo -e "${BLUE}[2/6]${NC} Установка зависимостей..."
apt install -y curl git ufw

# Установка Docker
echo -e "${BLUE}[3/6]${NC} Установка Docker..."
if ! command -v docker &> /dev/null; then
    curl -fsSL https://get.docker.com | bash
    systemctl enable docker
    systemctl start docker
else
    echo -e "${GREEN}[✓]${NC} Docker уже установлен"
fi

# Установка Docker Compose
if ! command -v docker-compose &> /dev/null; then
    apt install -y docker-compose-plugin
fi

# Скачивание PrivMsg
echo -e "${BLUE}[4/6]${NC} Скачивание PrivMsg..."
mkdir -p /opt
cd /opt

if [ -d "privmsg" ]; then
    echo -e "${YELLOW}Папка /opt/privmsg уже существует. Обновляем...${NC}"
    cd privmsg
    git pull || true
else
    git clone https://github.com/your-repo/privmsg.git || {
        echo -e "${YELLOW}Git clone не удался, создаём структуру вручную...${NC}"
        mkdir -p privmsg
        cd privmsg
    }
fi

cd /opt/privmsg

# Создание директорий
mkdir -p config/server config/coturn data/server

# Получаем внешний IP
EXTERNAL_IP=$(curl -s ifconfig.me || curl -s icanhazip.com || echo "YOUR_IP")
echo -e "${GREEN}[✓]${NC} Внешний IP: $EXTERNAL_IP"

# Создание конфигурации сервера
echo -e "${BLUE}[5/6]${NC} Создание конфигурации..."

cat > config/server/config.toml << EOF
[server]
host = "0.0.0.0"
port = 8443

[storage]
database_path = "./data/privmsg.db"
files_path = "./data/files"
max_message_age_hours = 168
max_file_age_hours = 72
cleanup_interval_minutes = 60

[turn]
enabled = true
urls = ["turn:$EXTERNAL_IP:3478", "turns:$EXTERNAL_IP:5349"]
username = "privmsg"
credential = "$TURN_PASSWORD"
credential_type = "password"
ttl_seconds = 86400

[admin]
master_key = "$ADMIN_KEY"

[limits]
max_file_size_mb = 100
max_message_size_kb = 64
max_pending_messages = 10000
rate_limit_messages_per_minute = 120
EOF

# Создание конфигурации TURN
cat > config/coturn/turnserver.conf << EOF
listening-port=3478
tls-listening-port=5349
realm=privmsg
fingerprint
lt-cred-mech
user=privmsg:$TURN_PASSWORD
min-port=49152
max-port=49200
no-cli
no-tcp-relay
external-ip=$EXTERNAL_IP
verbose
log-file=stdout
EOF

# Создание docker-compose.yml если нет
if [ ! -f "docker-compose.yml" ]; then
cat > docker-compose.yml << 'EOF'
version: '3.8'

services:
  privmsg-server:
    image: ghcr.io/your-repo/privmsg-server:latest
    container_name: privmsg-server
    restart: unless-stopped
    ports:
      - "8443:8443"
    volumes:
      - ./data/server:/app/data
      - ./config/server/config.toml:/app/config.toml:ro
    environment:
      - RUST_LOG=privmsg_server=info
    networks:
      - privmsg-network

  coturn:
    image: coturn/coturn:latest
    container_name: privmsg-turn
    restart: unless-stopped
    network_mode: host
    volumes:
      - ./config/coturn/turnserver.conf:/etc/coturn/turnserver.conf:ro
    command: -c /etc/coturn/turnserver.conf

networks:
  privmsg-network:
    driver: bridge
EOF
fi

# Настройка файрвола
echo -e "${BLUE}[6/6]${NC} Настройка файрвола..."
ufw allow 22/tcp      # SSH
ufw allow 8443/tcp    # PrivMsg API
ufw allow 3478/tcp    # TURN TCP
ufw allow 3478/udp    # TURN UDP
ufw allow 5349/tcp    # TURN TLS
ufw allow 5349/udp    # TURN DTLS
ufw allow 49152:49200/udp  # Media ports
ufw --force enable

# Запуск сервисов
echo ""
echo -e "${BLUE}Запуск сервисов...${NC}"
docker-compose pull || true
docker-compose up -d

# Ожидание запуска
echo "Ожидание запуска сервера..."
sleep 5

# Проверка
if curl -s http://localhost:8443/health | grep -q "ok"; then
    echo -e "${GREEN}[✓]${NC} Сервер успешно запущен!"
else
    echo -e "${YELLOW}[!]${NC} Сервер запускается... Проверьте через минуту."
fi

# Финальный вывод
echo ""
echo -e "${GREEN}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║              УСТАНОВКА ЗАВЕРШЕНА!                             ║${NC}"
echo -e "${GREEN}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}=== ВАЖНАЯ ИНФОРМАЦИЯ (СОХРАНИТЕ!) ===${NC}"
echo ""
echo -e "Внешний IP сервера: ${GREEN}$EXTERNAL_IP${NC}"
echo -e "Порт API: ${GREEN}8443${NC}"
echo ""
echo -e "Admin Key: ${GREEN}$ADMIN_KEY${NC}"
echo -e "TURN Password: ${GREEN}$TURN_PASSWORD${NC}"
echo ""
echo -e "${YELLOW}=== СЛЕДУЮЩИЕ ШАГИ ===${NC}"
echo ""
echo "1. Создайте первого пользователя:"
echo -e "   ${BLUE}docker exec -it privmsg-server ./privmsg-server generate-key --admin-key \"$ADMIN_KEY\"${NC}"
echo ""
echo "2. Для подключения клиентов используйте:"
echo -e "   Server: ${GREEN}$EXTERNAL_IP:8443${NC}"
echo ""
echo "3. Проверка статуса:"
echo -e "   ${BLUE}docker-compose ps${NC}"
echo -e "   ${BLUE}docker-compose logs -f${NC}"
echo ""
echo -e "${YELLOW}=== РЕКОМЕНДАЦИИ ===${NC}"
echo ""
echo "- Настройте домен и SSL для безопасности"
echo "- Регулярно делайте бэкапы: /opt/privmsg/data"
echo "- Храните Admin Key в безопасном месте"
echo ""

# Сохраняем ключи в файл
cat > /opt/privmsg/CREDENTIALS.txt << EOF
PrivMsg Server Credentials
==========================
Generated: $(date)

Server IP: $EXTERNAL_IP
Server Port: 8443

Admin Key: $ADMIN_KEY
TURN Password: $TURN_PASSWORD

KEEP THIS FILE SECURE!
EOF

chmod 600 /opt/privmsg/CREDENTIALS.txt
echo -e "Ключи также сохранены в: ${GREEN}/opt/privmsg/CREDENTIALS.txt${NC}"
echo ""
