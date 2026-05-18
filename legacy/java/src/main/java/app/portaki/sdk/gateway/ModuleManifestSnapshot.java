package app.portaki.sdk.gateway;

import java.util.List;
import java.util.Objects;

public record ModuleManifestSnapshot(
    String moduleId,
    List<String> scopes,
    List<String> publishedEventNames,
    List<String> queryNames,
    List<String> commandNames) {

  public ModuleManifestSnapshot {
    Objects.requireNonNull(moduleId);
    scopes = List.copyOf(scopes);
    publishedEventNames = List.copyOf(publishedEventNames);
    queryNames = List.copyOf(queryNames);
    commandNames = List.copyOf(commandNames);
  }

  public static ModuleManifestSnapshot of(
      String moduleId,
      List<String> scopes,
      List<String> publishedEventNames,
      List<String> queryNames,
      List<String> commandNames) {
    return new ModuleManifestSnapshot(moduleId, scopes, publishedEventNames, queryNames, commandNames);
  }
}
