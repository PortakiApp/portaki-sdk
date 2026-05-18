package app.portaki.sdk.email;

import java.time.Instant;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;

/**
 * Contexte lecture seule passé au composer {@link PortakiModuleEmail} lorsqu’un envoi est dû.
 * Données séjour / invité / config module — sans dépendance Spring.
 */
public record StayEmailContext(
        UUID stayId,
        UUID tenantId,
        UUID propertyId,
        String tenantGuestSlug,
        String stayAccessCode,
        Instant checkInAt,
        Instant checkOutAt,
        Optional<String> guestEmail,
        String guestLocale,
        Map<String, Object> moduleConfig) {

    public StayEmailContext {
        if (stayId == null) {
            throw new IllegalArgumentException("stay_email_context_stay_id_null");
        }
        if (tenantId == null) {
            throw new IllegalArgumentException("stay_email_context_tenant_id_null");
        }
        if (propertyId == null) {
            throw new IllegalArgumentException("stay_email_context_property_id_null");
        }
        if (tenantGuestSlug == null || tenantGuestSlug.isBlank()) {
            throw new IllegalArgumentException("stay_email_context_guest_slug_blank");
        }
        if (stayAccessCode == null || stayAccessCode.isBlank()) {
            throw new IllegalArgumentException("stay_email_context_access_code_blank");
        }
        if (checkInAt == null) {
            throw new IllegalArgumentException("stay_email_context_check_in_null");
        }
        if (checkOutAt == null) {
            throw new IllegalArgumentException("stay_email_context_check_out_null");
        }
        guestEmail = guestEmail == null ? Optional.empty() : guestEmail;
        if (guestLocale == null || guestLocale.isBlank()) {
            guestLocale = "fr";
        }
        moduleConfig = moduleConfig == null ? Map.of() : Map.copyOf(moduleConfig);
    }
}
