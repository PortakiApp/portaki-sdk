package app.portaki.sdk.event;

import java.time.Instant;
import java.util.UUID;

/** Payload shape for StayCreatedEvent (aligned with domain event). */
public record StayCreatedEvent(
    UUID stayId,
    UUID tenantId,
    UUID propertyId,
    Instant checkinAt,
    String guestEmail
) {}
