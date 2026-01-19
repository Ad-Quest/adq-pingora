# AdQuest API Server Configuration
# Similar to nginx server block

server {
    listen 80;
    listen 443 ssl http2;
    server_name api.ad-quest.ru;
    
    # SSL Configuration
    ssl_certificate /etc/letsencrypt/live/api.ad-quest.ru/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.ad-quest.ru/privkey.pem;
    
    # Challenge Engine API
    location /challenge {
        proxy_pass challenge_api;
        rate_limit 50 100;  # 50 rps, burst 100
        cors_enable;
    }
    
    # Billing Engine API  
    location /billing {
        proxy_pass billing_api;
        rate_limit 30 60;   # 30 rps, burst 60
        cors_enable;
    }
    
    # ERIR Integration API
    location /erir {
        proxy_pass erir_api;
        rate_limit 20 40;   # 20 rps, burst 40
        cors_enable;
    }
    
    # Shared Services / T-Bank API
    location /shared {
        proxy_pass shared_api;
        rate_limit 40 80;   # 40 rps, burst 80
        cors_enable;
    }
    
    location /tbank {
        proxy_pass shared_api;
        rate_limit 40 80;   # 40 rps, burst 80
        cors_enable;
    }
    
    # Health check (no rate limiting)
    location /health {
        proxy_pass shared_api;
        cors_enable;
    }
    
    # General API (load balanced)
    location / {
        proxy_pass core_api;
        rate_limit 10 20;   # 10 rps, burst 20
        cors_enable;
    }
}

# Upstream definitions
upstream challenge_api {
    server 127.0.0.1:8080;
}

upstream billing_api {
    server 127.0.0.1:8081;
}

upstream erir_api {
    server 127.0.0.1:8082;
}

upstream shared_api {
    server 127.0.0.1:8083;
}

# Load balanced upstream
upstream core_api {
    server 127.0.0.1:8080;
    server 127.0.0.1:8081;
    server 127.0.0.1:8082;
    server 127.0.0.1:8083;
}