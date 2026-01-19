# AdQuest Auth Server Configuration (Zitadel)
# Similar to nginx server block

server {
    listen 80;
    listen 443 ssl http2;
    server_name auth.ad-quest.ru;
    
    # SSL Configuration
    ssl_certificate /etc/letsencrypt/live/auth.ad-quest.ru/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/auth.ad-quest.ru/privkey.pem;
    
    # All requests go to Zitadel
    location / {
        proxy_pass zitadel_auth;
        rate_limit 10 20;   # 10 rps, burst 20
    }
}

# Upstream definition
upstream zitadel_auth {
    server 127.0.0.1:8091;
}