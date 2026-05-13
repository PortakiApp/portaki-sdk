package app.portaki.sdk.module.backend.http;

import java.net.InetAddress;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.time.Duration;

import app.portaki.sdk.module.backend.ModuleBackendException;

/**
 * GET HTTPS en texte UTF-8 avec garde-fous SSRF (réseaux privés, taille max) pour backends de modules hôte.
 */
public final class SafeHttpsUtf8Fetcher {

    private static final int MAX_BODY_CHARS = 512_000;

    private static final HttpClient HTTP_CLIENT =
            HttpClient.newBuilder()
                    .connectTimeout(Duration.ofSeconds(10))
                    .followRedirects(HttpClient.Redirect.NEVER)
                    .build();

    private SafeHttpsUtf8Fetcher() {}

    public static String fetch(String url) {
        URI uri;
        try {
            uri = URI.create(url.trim());
        } catch (IllegalArgumentException e) {
            throw new ModuleBackendException("ical_url_invalid", "invalid url", e);
        }
        if (!"https".equalsIgnoreCase(uri.getScheme())) {
            throw new ModuleBackendException("ical_url_https_only", "https only");
        }
        if (uri.getHost() == null || uri.getHost().isBlank()) {
            throw new ModuleBackendException("ical_url_host_missing", "host missing");
        }
        assertHostNotPrivate(uri.getHost());
        HttpRequest req =
                HttpRequest.newBuilder(uri)
                        .timeout(Duration.ofSeconds(25))
                        .header("User-Agent", "PortakiIcalSync/1.0")
                        .GET()
                        .build();
        try {
            HttpResponse<String> resp =
                    HTTP_CLIENT.send(req, HttpResponse.BodyHandlers.ofString(StandardCharsets.UTF_8));
            if (resp.statusCode() >= 300) {
                throw new ModuleBackendException("ical_http_status_" + resp.statusCode(), "http error");
            }
            String body = resp.body();
            if (body.length() > MAX_BODY_CHARS) {
                throw new ModuleBackendException("ical_body_too_large", "body too large");
            }
            return body;
        } catch (ModuleBackendException e) {
            throw e;
        } catch (Exception e) {
            throw new ModuleBackendException("ical_fetch_failed", "fetch failed", e);
        }
    }

    private static void assertHostNotPrivate(String host) {
        String h = host.toLowerCase();
        if ("localhost".equals(h)
                || h.endsWith(".localhost")
                || "0.0.0.0".equals(h)
                || "[::1]".equals(h)) {
            throw new ModuleBackendException("ical_url_host_forbidden", "forbidden host");
        }
        try {
            InetAddress addr = InetAddress.getByName(host);
            if (addr.isAnyLocalAddress()
                    || addr.isLoopbackAddress()
                    || addr.isLinkLocalAddress()
                    || addr.isSiteLocalAddress()
                    || addr.isMulticastAddress()) {
                throw new ModuleBackendException("ical_url_host_forbidden", "forbidden host");
            }
        } catch (ModuleBackendException e) {
            throw e;
        } catch (Exception e) {
            throw new ModuleBackendException("ical_dns_failed", "dns failed", e);
        }
    }
}
