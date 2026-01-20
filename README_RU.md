# PrivMsg - Приватный Self-Hosted Мессенджер

**PrivMsg** — это приватный мессенджер со сквозным шифрованием (E2EE), предназначенный для развёртывания на собственном сервере. Вы полностью контролируете свои данные — никаких третьих сторон.

## Содержание

1. [Возможности](#возможности)
2. [Архитектура](#архитектура)
3. [Требования](#требования)
4. [Быстрый старт с Docker](#быстрый-старт-с-docker)
5. [Полное руководство по развёртыванию на VPS](#полное-руководство-по-развёртыванию-на-vps)
6. [Настройка HTTPS](#настройка-https)
7. [Управление пользователями](#управление-пользователями)
8. [Сборка из исходников](#сборка-из-исходников)
9. [Настройка клиентов](#настройка-клиентов)
10. [API Reference](#api-reference)
11. [Устранение неполадок](#устранение-неполадок)
12. [Безопасность](#безопасность)

---

## Возможности

- **Сквозное шифрование (E2EE)**: Обмен ключами X25519 + шифрование AES-256-GCM
- **Self-Hosted**: Полный контроль над данными и сервером
- **Мультиплатформенность**: Клиенты для Windows, Linux, Android
- **Голосовые и видеозвонки**: На базе WebRTC с поддержкой TURN сервера
- **Голосовые и видеосообщения**: Отправка зашифрованных медиа-сообщений
- **Передача файлов**: Зашифрованный обмен файлами
- **Анонимность**: Не требуется телефон или email — только ключ доступа

---

## Архитектура

```
┌─────────────────────────────────────────────────────────────┐
│                    КЛИЕНТСКИЕ ПРИЛОЖЕНИЯ                    │
├─────────────────────────────────────────────────────────────┤
│
│  ┌──────────────────┐      ┌──────────────────┐
│  │ Desktop клиент   │      │  Android клиент  │
│  │   (Rust/iced)    │      │    (Kotlin)      │
│  └────────┬─────────┘      └────────┬─────────┘
│           │                         │
│           │  E2EE Сообщения        │
│           │  (X25519 + AES-256)    │
│           │                         │
└───────────┼─────────────────────────┼─────────────────────────┘
            │                         │
            └───────────┬─────────────┘
                        │
        ┌───────────────▼────────────────┐
        │   PrivMsg Сервер (Rust/Axum)   │
        │   • Ретрансляция сообщений     │
        │   • Управление пользователями  │
        │   • Хранение файлов            │
        └────────────────────────────────┘
                        │
            ┌───────────▼───────────┐
            │   TURN Сервер         │
            │   (coturn)            │
            │   Ретрансляция WebRTC │
            └───────────────────────┘
```

### Как это работает

1. **Сервер** — минимальный relay-сервер. Он НЕ может читать ваши сообщения, так как всё шифрование происходит на клиентах.

2. **TURN сервер** — нужен для видео/аудио звонков через WebRTC, особенно когда клиенты находятся за NAT.

3. **Клиенты** — выполняют всё шифрование локально. Ключи шифрования никогда не покидают устройство.

---

## Требования

### Для сервера (VPS)

| Параметр | Минимум | Рекомендуется |
|----------|---------|---------------|
| RAM | 512 MB | 2 GB |
| CPU | 1 vCPU | 2 vCPU |
| Диск | 5 GB SSD | 20 GB SSD |
| ОС | Debian 12+ / Ubuntu 22.04+ | Ubuntu 24.04 |

### Необходимые порты

| Порт | Протокол | Назначение |
|------|----------|------------|
| 9443 | TCP | API сервер |
| 3478 | TCP/UDP | TURN (обычный) |
| 5349 | TCP/UDP | TURNS (TLS) |
| 49152-49200 | UDP | TURN media relay |

### Для локальной разработки

- Docker и Docker Compose
- Или Rust 1.75+ (для сборки из исходников)

---

## Быстрый старт с Docker

Это самый простой способ запустить сервер для тестирования.

### Шаг 1: Клонирование репозитория

```bash
git clone https://github.com/sssilverhand/secure-messenger.git
cd secure-messenger
```

### Шаг 2: Генерация секретных ключей

Перед запуском необходимо сгенерировать безопасные ключи:

```bash
# Генерация admin key (для управления пользователями)
openssl rand -base64 32
# Пример вывода: K7xR2mN9pQwE3sT6vY1zA8bC5dF0gH4jL

# Генерация TURN secret (для видеозвонков)
openssl rand -base64 32
# Пример вывода: M2nB9xP3qW7eR1tY5uI8oA4sD6fG0hJ
```

**ВАЖНО**: Сохраните эти ключи в безопасном месте! Они понадобятся для настройки.

### Шаг 3: Настройка конфигурации сервера

Откройте файл `config/server/config.toml` в текстовом редакторе:

```bash
nano config/server/config.toml
```

Найдите и измените следующие строки:

```toml
[admin]
master_key = "ВСТАВЬТЕ_СЮДА_ВАШ_ADMIN_KEY"

[turn]
credential = "ВСТАВЬТЕ_СЮДА_ВАШ_TURN_SECRET"
```

Сохраните файл (в nano: Ctrl+O, Enter, Ctrl+X).

### Шаг 4: Настройка TURN сервера

Откройте файл `config/coturn/turnserver.conf`:

```bash
nano config/coturn/turnserver.conf
```

Найдите строку `static-auth-secret` и замените:

```
static-auth-secret=ВСТАВЬТЕ_СЮДА_ВАШ_TURN_SECRET
```

**ВАЖНО**: TURN secret должен быть ОДИНАКОВЫМ в обоих файлах конфигурации!

### Шаг 5: Запуск сервера

```bash
docker compose up -d
```

Docker скачает необходимые образы и запустит контейнеры. Это может занять несколько минут при первом запуске.

### Шаг 6: Проверка работы

```bash
# Проверка статуса контейнеров
docker compose ps

# Проверка health endpoint
curl http://localhost:9443/health

# Ожидаемый ответ:
# {"status":"ok","timestamp":...,"version":"1.0.0"}
```

### Шаг 7: Создание первого пользователя

```bash
# Создание пользователя через API
curl -X POST http://localhost:9443/api/v1/admin/users \
  -H "Content-Type: application/json" \
  -d '{"admin_key":"ВАШ_ADMIN_KEY"}'

# Ответ будет содержать:
# {
#   "user_id": "AbCd1234",
#   "access_key": "очень_длинный_ключ_доступа..."
# }
```

**ВАЖНО**: Сохраните `user_id` и `access_key`! Ключ доступа показывается только один раз!

### Шаг 8: Остановка сервера

```bash
docker compose down
```

---

## Полное руководство по развёртыванию на VPS

Это подробное руководство для развёртывания на production сервере.

### Этап 1: Подготовка VPS

#### 1.1 Подключение к серверу

```bash
ssh root@ваш_ip_адрес
```

#### 1.2 Обновление системы

```bash
# Обновление списка пакетов
apt update

# Обновление установленных пакетов
apt upgrade -y

# Установка базовых утилит
apt install -y curl wget git nano ufw
```

#### 1.3 Создание пользователя (рекомендуется)

Работать от root небезопасно. Создайте отдельного пользователя:

```bash
# Создание пользователя
adduser privmsg

# Добавление в группу sudo
usermod -aG sudo privmsg

# Переключение на нового пользователя
su - privmsg
```

### Этап 2: Установка Docker

#### 2.1 Установка Docker Engine

```bash
# Скачивание и запуск установочного скрипта
curl -fsSL https://get.docker.com | sh

# Добавление текущего пользователя в группу docker
sudo usermod -aG docker $USER

# ВАЖНО: Выйдите и зайдите снова, чтобы изменения вступили в силу
exit
```

Подключитесь снова:
```bash
ssh privmsg@ваш_ip_адрес
```

#### 2.2 Проверка установки

```bash
# Проверка версии Docker
docker --version

# Проверка работы Docker
docker run hello-world
```

### Этап 3: Настройка файрвола

```bash
# Разрешение SSH (чтобы не потерять доступ!)
sudo ufw allow 22/tcp

# Разрешение портов PrivMsg
sudo ufw allow 9443/tcp    # API сервер
sudo ufw allow 3478/tcp    # TURN TCP
sudo ufw allow 3478/udp    # TURN UDP
sudo ufw allow 5349/tcp    # TURNS TCP
sudo ufw allow 5349/udp    # TURNS UDP

# Диапазон портов для медиа (WebRTC)
sudo ufw allow 49152:49200/udp

# Если планируете использовать HTTPS через Nginx
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp

# Включение файрвола
sudo ufw enable

# Проверка правил
sudo ufw status
```

### Этап 4: Клонирование и настройка проекта

#### 4.1 Клонирование репозитория

```bash
# Переход в домашнюю директорию
cd ~

# Клонирование
git clone https://github.com/sssilverhand/secure-messenger.git

# Переход в директорию проекта
cd secure-messenger
```

#### 4.2 Генерация секретных ключей

```bash
# Генерация и сохранение ключей
ADMIN_KEY=$(openssl rand -base64 32)
TURN_SECRET=$(openssl rand -base64 32)

# Вывод ключей (СОХРАНИТЕ ИХ!)
echo "=========================================="
echo "СОХРАНИТЕ ЭТИ КЛЮЧИ В БЕЗОПАСНОМ МЕСТЕ!"
echo "=========================================="
echo "Admin Key: $ADMIN_KEY"
echo "TURN Secret: $TURN_SECRET"
echo "=========================================="
```

#### 4.3 Настройка конфигурации сервера

```bash
# Редактирование конфигурации сервера
nano config/server/config.toml
```

Измените следующие параметры:

```toml
[server]
host = "0.0.0.0"
port = 9443

[storage]
database_path = "/app/data/privmsg.db"
files_path = "/app/data/files"
max_message_age_hours = 168          # 7 дней хранения сообщений
max_file_age_hours = 72              # 3 дня хранения файлов
cleanup_interval_minutes = 60

[turn]
enabled = true
# Замените localhost на ваш домен или IP
urls = ["turn:ваш_домен_или_ip:3478", "turns:ваш_домен_или_ip:5349"]
username = "privmsg"
credential = "ВСТАВЬТЕ_TURN_SECRET"
credential_type = "password"
ttl_seconds = 86400

[admin]
master_key = "ВСТАВЬТЕ_ADMIN_KEY"

[limits]
max_file_size_mb = 100
max_message_size_kb = 64
max_pending_messages = 10000
rate_limit_messages_per_minute = 120
```

Сохраните: Ctrl+O, Enter, Ctrl+X

#### 4.4 Настройка TURN сервера

```bash
nano config/coturn/turnserver.conf
```

Измените следующие параметры:

```
# Порты
listening-port=3478
tls-listening-port=5349

# IP адрес вашего сервера
listening-ip=0.0.0.0
external-ip=ВАШ_ВНЕШНИЙ_IP

# Домен (если есть)
realm=ваш_домен.com

# Секрет (ДОЛЖЕН СОВПАДАТЬ с config.toml!)
static-auth-secret=ВСТАВЬТЕ_TURN_SECRET

# Диапазон портов для медиа
min-port=49152
max-port=49200

# Логирование
log-file=/var/log/turnserver.log
verbose
```

Сохраните файл.

### Этап 5: Запуск сервисов

```bash
# Запуск в фоновом режиме
docker compose up -d

# Проверка статуса
docker compose ps

# Просмотр логов
docker compose logs -f
```

Нажмите Ctrl+C для выхода из просмотра логов.

### Этап 6: Проверка работы

```bash
# Проверка health endpoint
curl http://localhost:9443/health

# Проверка извне (замените на ваш IP)
curl http://ваш_ip:9443/health
```

### Этап 7: Создание пользователей

```bash
# Создание пользователя
curl -X POST http://localhost:9443/api/v1/admin/users \
  -H "Content-Type: application/json" \
  -d '{"admin_key":"ВАШ_ADMIN_KEY"}'
```

Пример ответа:
```json
{
  "user_id": "Bmo61cyW",
  "access_key": "UiJJrObUbiwzzCCSI4q7acCEqZ32yipC3jlRZwP5PSY"
}
```

Создайте столько пользователей, сколько нужно. Каждый пользователь получает уникальную пару `user_id` + `access_key`.

---

## Настройка HTTPS

Для production настоятельно рекомендуется использовать HTTPS.

### Вариант 1: Nginx + Let's Encrypt (рекомендуется)

#### 1.1 Установка Nginx и Certbot

```bash
sudo apt install -y nginx certbot python3-certbot-nginx
```

#### 1.2 Настройка Nginx

```bash
sudo nano /etc/nginx/sites-available/privmsg
```

Вставьте конфигурацию:

```nginx
server {
    listen 80;
    server_name ваш_домен.com;

    location / {
        proxy_pass http://127.0.0.1:9443;
        proxy_http_version 1.1;

        # WebSocket support
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";

        # Заголовки
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Таймауты для WebSocket
        proxy_connect_timeout 7d;
        proxy_send_timeout 7d;
        proxy_read_timeout 7d;
    }
}
```

#### 1.3 Активация конфигурации

```bash
# Создание символической ссылки
sudo ln -s /etc/nginx/sites-available/privmsg /etc/nginx/sites-enabled/

# Проверка конфигурации
sudo nginx -t

# Перезапуск Nginx
sudo systemctl reload nginx
```

#### 1.4 Получение SSL сертификата

```bash
sudo certbot --nginx -d ваш_домен.com
```

Следуйте инструкциям на экране. Certbot автоматически настроит HTTPS и перенаправление с HTTP.

#### 1.5 Автоматическое обновление сертификата

Certbot автоматически добавляет задачу в cron. Проверить можно так:

```bash
sudo certbot renew --dry-run
```

### Вариант 2: Прямой TLS на сервере

Если не хотите использовать Nginx, можно настроить TLS прямо в PrivMsg сервере.

#### 2.1 Получение сертификатов

```bash
# Установка certbot
sudo apt install -y certbot

# Получение сертификата (standalone режим)
sudo certbot certonly --standalone -d ваш_домен.com

# Сертификаты будут в:
# /etc/letsencrypt/live/ваш_домен.com/fullchain.pem
# /etc/letsencrypt/live/ваш_домен.com/privkey.pem
```

#### 2.2 Настройка сервера

Раскомментируйте и настройте секцию TLS в `config/server/config.toml`:

```toml
[tls]
cert_path = "/app/certs/fullchain.pem"
key_path = "/app/certs/privkey.pem"
```

#### 2.3 Монтирование сертификатов в Docker

Отредактируйте `docker-compose.yml`, добавив монтирование сертификатов:

```yaml
services:
  privmsg-server:
    volumes:
      - ./config/server:/app/config:ro
      - ./data/server:/app/data
      - /etc/letsencrypt/live/ваш_домен.com:/app/certs:ro
```

---

## Управление пользователями

### Создание пользователя

```bash
# Через API
curl -X POST http://localhost:9443/api/v1/admin/users \
  -H "Content-Type: application/json" \
  -d '{"admin_key":"ВАШ_ADMIN_KEY"}'

# Или через CLI (если сервер запущен локально)
docker exec -it privmsg-server ./privmsg-server generate-key --admin-key ВАШ_ADMIN_KEY
```

### Создание пользователя с определённым ID

```bash
curl -X POST http://localhost:9443/api/v1/admin/users \
  -H "Content-Type: application/json" \
  -d '{"admin_key":"ВАШ_ADMIN_KEY", "user_id":"custom_user_id"}'
```

### Список всех пользователей

```bash
docker exec -it privmsg-server ./privmsg-server list-keys --admin-key ВАШ_ADMIN_KEY
```

### Отзыв доступа пользователя

```bash
# Деактивация (пользователь больше не сможет войти)
docker exec -it privmsg-server ./privmsg-server revoke-key --admin-key ВАШ_ADMIN_KEY --user-id USER_ID

# Или полное удаление через API
curl -X DELETE "http://localhost:9443/api/v1/admin/users/USER_ID" \
  -H "Content-Type: application/json" \
  -d '{"admin_key":"ВАШ_ADMIN_KEY"}'
```

### Статистика сервера

```bash
curl -X GET http://localhost:9443/api/v1/admin/stats \
  -H "Content-Type: application/json" \
  -d '{"admin_key":"ВАШ_ADMIN_KEY"}'
```

Пример ответа:
```json
{
  "total_users": 5,
  "active_users": 5,
  "online_users": 2,
  "pending_messages": 15,
  "stored_files": 3,
  "storage_used_mb": 12.5
}
```

---

## Сборка из исходников

### Сборка сервера

#### Linux / macOS

```bash
# Установка Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Переход в директорию сервера
cd server

# Сборка release версии
cargo build --release

# Бинарный файл будет в: target/release/privmsg-server
```

#### Windows

1. Установите [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/downloads/) с компонентом "C++ build tools"
2. Установите Rust через [rustup](https://rustup.rs/)
3. Откройте PowerShell:

```powershell
cd server
cargo build --release

# Бинарный файл будет в: target\release\privmsg-server.exe
```

### Сборка Desktop клиента

#### Linux (Ubuntu/Debian)

```bash
# Установка зависимостей
sudo apt install -y build-essential pkg-config libssl-dev libgtk-3-dev \
    libasound2-dev cmake libfontconfig1-dev

# Сборка
cd desktop
cargo build --release

# Бинарный файл: target/release/privmsg-desktop
```

#### Windows

```powershell
cd desktop
cargo build --release

# Бинарный файл: target\release\privmsg-desktop.exe
```

### Сборка Android клиента

#### Требования

- Android Studio или Android SDK Command Line Tools
- JDK 17+

#### Сборка APK

```bash
cd android

# Linux/macOS
./gradlew assembleRelease

# Windows
gradlew.bat assembleRelease

# APK будет в: app/build/outputs/apk/release/app-release.apk
```

---

## Настройка клиентов

### Desktop клиент

1. Скачайте клиент из [Releases](../../releases) или соберите из исходников
2. Запустите приложение
3. На экране входа введите:
   - **Server URL**: `https://ваш_домен.com` или `http://ваш_ip:9443`
   - **User ID**: Полученный от администратора
   - **Access Key**: Полученный от администратора
4. Нажмите "Connect"

### Android клиент

1. Скачайте APK из [Releases](../../releases) или соберите из исходников
2. Установите APK (может потребоваться разрешить установку из неизвестных источников)
3. На экране входа введите:
   - **Server URL**: `https://ваш_домен.com` или `http://ваш_ip:9443`
   - **User ID**: Полученный от администратора
   - **Access Key**: Полученный от администратора
4. Нажмите "Войти"

### Первый запуск

При первом входе клиент:
1. Генерирует пару ключей шифрования (X25519)
2. Отправляет публичный ключ на сервер
3. Создаёт локальную базу данных для хранения сообщений

**Ключи шифрования хранятся только на вашем устройстве!**

---

## API Reference

### Аутентификация

#### POST /api/v1/auth/login

Вход в систему.

**Запрос:**
```json
{
  "user_id": "AbCd1234",
  "access_key": "ваш_ключ_доступа",
  "device_name": "Мой телефон",
  "device_type": "android",
  "device_public_key": "base64_encoded_public_key"
}
```

**Ответ:**
```json
{
  "token": "session_token",
  "device_id": "device_id",
  "expires_at": 1234567890,
  "user": {
    "user_id": "AbCd1234",
    "display_name": null,
    "avatar_file_id": null,
    "public_key": "...",
    "last_seen_at": "2024-01-01 12:00:00"
  }
}
```

#### POST /api/v1/auth/refresh

Обновление токена сессии.

#### POST /api/v1/auth/logout

Выход из системы.

### Пользователи

#### GET /api/v1/users/me

Получение информации о текущем пользователе.

**Заголовки:**
```
Authorization: Bearer <token>
```

#### GET /api/v1/users/:user_id

Получение публичного профиля пользователя.

#### POST /api/v1/users/me/profile

Обновление профиля.

**Запрос:**
```json
{
  "display_name": "Моё имя",
  "public_key": "base64_encoded_public_key"
}
```

### WebSocket

#### Подключение

```
ws://сервер:9443/ws
wss://сервер/ws  (для HTTPS)
```

#### Аутентификация после подключения

```json
{
  "type": "authenticate",
  "payload": {
    "token": "session_token"
  }
}
```

#### Отправка сообщения

```json
{
  "type": "message",
  "payload": {
    "message_id": "уникальный_uuid",
    "sender_id": "ваш_user_id",
    "recipient_id": "user_id_получателя",
    "encrypted_content": "base64_зашифрованные_данные",
    "message_type": "text",
    "timestamp": 1234567890
  }
}
```

#### Типы сообщений

- `text` — текстовое сообщение
- `voice` — голосовое сообщение
- `video` — видеосообщение
- `image` — изображение
- `file` — файл
- `call_signal` — сигнал звонка

---

## Устранение неполадок

### Сервер не запускается

```bash
# Проверка логов
docker compose logs privmsg-server

# Частые причины:
# 1. Порт занят
netstat -tlnp | grep 9443

# 2. Ошибка в конфигурации
cat config/server/config.toml | grep -v "^#" | grep -v "^$"

# 3. Проблемы с правами на директорию data
ls -la data/
```

### Звонки не работают

```bash
# Проверка TURN сервера
docker compose logs coturn

# Проверка портов
nc -zvu ваш_ip 3478
nc -zvw 5 ваш_ip 5349

# Проверка файрвола
sudo ufw status

# Частая проблема: разные секреты в config.toml и turnserver.conf
grep -i secret config/server/config.toml
grep -i secret config/coturn/turnserver.conf
```

### Клиент не подключается

1. **Проверьте URL сервера** — должен включать протокол (http:// или https://)
2. **Проверьте порты** — `nc -zv сервер 9443`
3. **Проверьте сертификат** — для HTTPS убедитесь, что сертификат валидный
4. **Проверьте user_id и access_key** — они чувствительны к регистру

### WebSocket отключается

```bash
# Проверка Nginx (если используется)
sudo nginx -t

# Проверка настроек прокси для WebSocket
grep -A5 "Upgrade" /etc/nginx/sites-available/privmsg
```

Убедитесь, что в Nginx есть эти строки:
```nginx
proxy_set_header Upgrade $http_upgrade;
proxy_set_header Connection "upgrade";
```

### Сброс базы данных

```bash
# Остановка сервера
docker compose down

# Удаление данных
rm -rf data/server/*

# Запуск сервера (БД создастся заново)
docker compose up -d
```

---

## Безопасность

### Шифрование

- **Обмен ключами**: X25519 (Curve25519) — современный алгоритм на эллиптических кривых
- **Шифрование сообщений**: AES-256-GCM — аутентифицированное шифрование
- **Хеширование**: SHA-256

### Что сервер НЕ может делать

- Читать содержимое сообщений
- Видеть файлы (они зашифрованы)
- Восстановить ключи шифрования

### Что сервер МОЖЕТ видеть

- Метаданные: кто с кем общается, когда
- Размеры сообщений и файлов
- IP адреса клиентов

### Рекомендации по безопасности

1. **Используйте HTTPS** — всегда в production
2. **Меняйте секреты** — никогда не используйте значения по умолчанию
3. **Обновляйте сервер** — следите за обновлениями безопасности
4. **Делайте бэкапы** — регулярно копируйте базу данных
5. **Мониторьте логи** — отслеживайте подозрительную активность
6. **Ограничьте доступ** — используйте файрвол

### Бэкап базы данных

```bash
# Создание бэкапа
docker exec privmsg-server cp /app/data/privmsg.db /app/data/backup_$(date +%Y%m%d).db

# Копирование на локальную машину
docker cp privmsg-server:/app/data/backup_*.db ./backups/
```

---

## Обновление сервера

```bash
# Переход в директорию проекта
cd ~/secure-messenger

# Получение обновлений
git pull

# Пересборка и перезапуск
docker compose down
docker compose build --no-cache
docker compose up -d

# Проверка
docker compose logs -f
```

---

## Поддержка

- **Issues**: [GitHub Issues](../../issues)
- **Discussions**: [GitHub Discussions](../../discussions)

---

## Лицензия

Этот проект распространяется под лицензией GNU General Public License v3.0 — см. файл [LICENSE](LICENSE).

---

*PrivMsg — ваша приватная связь, ваши правила.*
