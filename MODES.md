# Downzer - Modos de Operación

## Overview

Downzer soporta múltiples modos de operación para diferentes tipos de tareas de fuzzing y escaneo. Cada modo es optimizado para su caso de uso específico.

## Modos Disponibles

### 1. Download Mode (Por defecto)
Descarga archivos desde URLs con patrón de fuzzing.

```bash
# Descarga con rango
downzer "https://site.com/file-FUZZR.jpg" -m download -r 0-10 -o ./downloads

# Descarga con wordlist
downzer "https://site.com/FUZZW1" -m download -w malware.txt -o ./files

# Combinaciones: rango + wordlist
downzer "https://site.com/api/FUZZW1/FUZZW2/data-FUZZR"-m download -r 0-5 -w "common.txt:user.txt"  -o ./api_data
```

**Opciones específicas:**
- `-o, --outdir`: Directorio de salida (por defecto: `.`)
- `--dd, --download-body`: Descargar cuerpo de respuesta HTTP (incluso si no es archivo)
- `-c, --content-type`: Filtrar por tipo MIME (ej: `image,video,pdf`)

---

### 2. Web Request Mode
Envía peticiones HTTP con múltiples métodos y manejo de respuestas.

```bash
# GET requests
downzer "https://site.com/api/FUZZW1" -m webrequest -w "admin:panel:user" --method GET -vv

# POST requests
downzer "https://api.com/endpoint" -m webrequest -r 0-100 --method POST --data '{"id": "FUZZR"}' -vv

# PUT requests  
downzer "https://api.com/user/FUZZW1" -m webrequest -w userids.txt --method PUT --data-file update.json

# PATCH requests
downzer "https://api.com/item/FUZZR" -m webrequest -r 1-50 --method PATCH --data '{"status": "active"}'

# DELETE requests
downzer "https://api.com/resource/FUZZW1" -m webrequest -w "old_:legacy_" --method DELETE -vv

# HEAD requests (solo headers, sin descargar cuerpo)
downzer "https://cdn.com/asset-FUZZR.zip" -m webrequest -r 0-1000 --method HEAD
```

**Opciones específicas:**
- `--method`: HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
- `--data`: Datos en el cuerpo (POST, PUT, PATCH)
- `--data-file`: Archivo con datos para el cuerpo
- `--dd`: Descargar cuerpo de respuesta
- `-vv`: Verbosidad alta para ver todas las peticiones

**Verbosity:**
- `-q`: Sin salida (solo errores)
- Normal: Resumen final
- `-v`: Errores durante ejecución
- `-vv`: Todas las peticiones con status codes
- `-vvv`: Debug detallado

---

### 3. Port Scan Mode (En desarrollo)
Escaneo de puertos con técnicas SYN/ACK.

```bash
# Escaneo básico
downzer "target-host:22,80,443,3306,5432" -m portscan -w "192.168.1.1:192.168.1.254" -vv

# Con rango de puertos
downzer "target-ip:FUZZR" -m portscan -r 1-1000 --timeout 5

# Sin DNS
downzer "FUZZW1:22,80,443" -m portscan -w ips.txt --nodns
```

**Estado:** Requiere raw sockets, no implementado aún.

---

### 4. Network Protocols (SSH, FTP, Telnet, Mail) (En desarrollo)
Conexiones a protocolos de red específicos.

```bash
# SSH
downzer "target-host:22 -m ssh -w "user1:user2:admin"" --timeout 10

# FTP  
downzer "ftp-server:21" -m ftp -w "users.txt" --timeout 30

# Telnet
downzer "server:FUZZR" -m telnet -r 0-5 --timeout 5

# IMAP (Email)
downzer "FUZZW1:993" -m imap -w "mailserver.txt" --timeout 15

# SMTP
downzer "FUZZW1:25" -m smtp -w "smtp-servers.txt" --timeout 10

# POP3
downzer "FUZZW1:110" -m pop3 -w "mail-hosts.txt"
```

**Estado:** Protocolos no implementados aún, requieren crates especializados.

---

## Opciones Globales

### Fuzzing

```bash
# Rango numérico (reemplaza FUZZR)
-r 0-100
-r 0-1000000

# Wordlists (reemplaza FUZZW1, FUZZW2, etc)
-w malware.txt
-w "admin:test:user"  # CSV inline
-w "list1.txt:list2.txt"  # Múltiples listas

# Combinación de rango + wordlist
downzer "https://api.com/user/FUZZW1/role/FUZZW2/page/FUZZR" -r 0-10 -w "users.txt:roles.txt"

# Parallelismo (iteración sincronizada)
--parallel

# Shuffling aleatorio
--random

# Exclusiones
-e "admin,root,system,guest"
```

### Rendimiento

```bash
# Concurrencia (por defecto: 20)
--max-concurrent 50
--max-concurrent 100  # Más agresivo

# Delay entre peticiones
-d 100ms   # 100 milisegundos entre cada petición
-d "5x10"  # Pausa de 5ms cada 10 peticiones

# Timeout por petición (por defecto: 30s)
--timeout 60
--timeout 5    # Para escaneos rápidos
```

### Network

```bash
# Proxy HTTP/SOCKS5
--proxy "http://proxy.company.com:8080"
--proxy "socks5://proxy.local:1080"

# Desactivar DNS (usar IPs directas)
--nodns
-n

# MAC Address personalizado
--mac "00:11:22:33:44:55"
--random-mac    # Aleatorio en cada petición
--mac "macs.txt"  # Una MAC por línea

# User-Agent personalizado
--ua "Mozilla/5.0 Custom"
--random-ua      # UA aleatorio en cada petición
--ua "agents.txt"  # Un UA por línea
```

### Output

```bash
# Silencioso
-q, --quiet

# Verbosidad (puede apilarse)
-v   # Info básica
-vv  # Detalles por petición
-vvv # Debug completo

# Directorio de salida
-o ./resultados
--outdir /tmp/scan_results

# Logging
--log                  # Habilitar logging
--log-dir ./logs       # Directorio de logs
```

### Configuración

```bash
# Panel de configuración interactivo
downzer config

# Debug mode
--debug

# Agregar como tarea en background
--add

# Agregar a cola (esperar tareas actuales)
--queue
```

---

## Ejemplos Completos

### Ejemplo 1: Fuzzing de endpoints API

```bash
downzer "https://api.example.com/v1/FUZZW1/FUZZR" \
  -m webrequest \
  -r 0-9999 \
  -w "users:posts:comments:messages" \
  --method GET \
  -vv \
  --max-concurrent 50 \
  --timeout 10
```

### Ejemplo 2: Descarga masiva con filtros

```bash
downzer "https://cdn.example.com/backup-FUZZR.zip" \
  -m download \
  -r 0-100000 \
  -o ./backups \
  -c "application/zip" \
  -d "50ms" \
  --max-concurrent 10
```

### Ejemplo 3: Fuzzing de rutas con múltiples parámetros

```bash
downzer "https://app.local/panel/FUZZW1/action/FUZZW2" \
  -m webrequest \
  -w "admin:user:manager:moderator" \
  -w "view:edit:delete:export" \
  --method POST \
  --data '{"action": "FUZZW2"}' \
  --timeout 30 \
  -vv \
  --max-concurrent 20
```

### Ejemplo 4: API testing con User-Agents

```bash
downzer "https://api.example.com/endpoint?id=FUZZR" \
  -m webrequest \
  -r 1-100 \
  --method GET \
  --ua "agents.txt" \
  --random-ua \
  --max-concurrent 30 \
  -v
```

### Ejemplo 5: Descarga con proxy y DNS deshabilitado

```bash
downzer "https://192.168.1.100/resource-FUZZR.pdf" \
  -m download \
  -r 0-50 \
  --proxy "http://corporate-proxy:8080" \
  --nodns \
  --timeout 20 \
  -o ./secure_downloads
```

---

## Parámetros de Template

En la URL/objetivo puedes usar estos placeholders:

| Placeholder | Descripción | Ejemplo |
|-----------|-----------|---------|
| `FUZZR` | Números de rango (-r) | `https://site.com/file-FUZZR` |
| `FUZZW1` | Primera wordlist (-w) | `https://site.com/FUZZW1` |
| `FUZZW2` | Segunda wordlist (-w) | `https://site.com/api/FUZZW1/FUZZW2` |
| `FUZZW3` | Tercera wordlist | Etc... |

**Nota:** Puedes combinar múltiples placeholders en una misma URL.

---

## Salida y Resultados

Cada modo proporciona salida formateada:

```
╔════════════════════════════════════════╗
║    Downzer - Resource Fuzzer/Download ║
╚════════════════════════════════════════╝
[*] Processing range: 0-100
[*] Processing 1 wordlist(s)
[*] Generating combinations...
  Total combinations: 101
[*] Processing URL template
  Total URLs to download: 101

[*] Task #1 started
[*] 101 URLs to download from https://example.com/file-FUZZR

[*] Modo: Descarga (101 URLs)
  Concurrencia: 20
  Timeout: 30s

═══════════════════════════════════════
[✓] Task #1 completed
  Mode: download (101)
  Successful: 87
  Failed:     14
  Details: Descargados: 87, Ignorados: 10, No encontrados: 4, Errores: 0, Bytes: 2547632
═══════════════════════════════════════
[*] Cleaning up...
[✓] Done!
```

---

## Comandos Adicionales

```bash
# Listar tareas activas
downzer list

# Pausar una tarea
downzer pause 1 2 3

# Reanudar una tarea pausada
downzer resume 1

# Detener una tarea
downzer stop 1

# Panel de configuración
downzer config
```

---

## Notas Importantes

1. **Verbosidad:** Por defecto, solo se muestra resumen. Usa `-v` o `-vv` para más detalles.
2. **Concurrencia:** El default de 20 conexiones simultáneas es generalmente seguro. Aumenta según la capacidad del target.
3. **Timeout:** Por defecto 30 segundos. Reduce para targets lentos, aumenta para operaciones pesadas.
4. **Proxy:** Compatible con HTTP y SOCKS5. Requiere proxy válido.
5. **DNS:** Desactivar con `--nodns` mejora velocidad cuando ya conoces IPs.
6. **MAC/UA:** Requieren archivos con una entrada por línea o valores CSV.

