package app.portaki.sdk.module.backend.run;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.ArrayList;
import java.util.List;
import java.util.UUID;

import org.junit.jupiter.api.Test;

import app.portaki.sdk.module.backend.ModuleBackendException;
import app.portaki.sdk.module.backend.ModuleHostContext;

class ModuleRunPipelineTest {

    private static final class Carry {
        int value;
    }

    @Test
    void execute_runsStepsInOrder_andNotifiesListener() {
        List<String> events = new ArrayList<>();
        ModuleHostContext host = new ModuleHostContext(UUID.randomUUID(), UUID.randomUUID(), "test-module");
        ModuleRunContext ctx = ModuleRunContext.start(host);
        Carry carry = new Carry();
        ModuleRunListener listener =
                new ModuleRunListener() {
                    @Override
                    public void onRunStarted(ModuleRunContext c) {
                        events.add("start");
                    }

                    @Override
                    public void onStepStarted(ModuleRunContext c, String stepId) {
                        events.add("step:" + stepId);
                    }

                    @Override
                    public void onStepFinished(ModuleRunContext c, String stepId, ModuleRunStepResult result) {
                        events.add("done:" + stepId + ":" + result.ok());
                    }

                    @Override
                    public void onRunFinished(ModuleRunContext c, ModuleRunReport report) {
                        events.add("finish");
                    }
                };
        ModuleRunPipeline<Carry> pipeline =
                ModuleRunPipeline.<Carry>of(
                                new ModuleRunStep<>() {
                                    @Override
                                    public String id() {
                                        return "a";
                                    }

                                    @Override
                                    public ModuleRunStepResult run(ModuleRunContext c, Carry k) {
                                        k.value += 1;
                                        return ModuleRunStepResult.ok("a", "ok");
                                    }
                                },
                                new ModuleRunStep<>() {
                                    @Override
                                    public String id() {
                                        return "b";
                                    }

                                    @Override
                                    public ModuleRunStepResult run(ModuleRunContext c, Carry k) {
                                        k.value += 10;
                                        return ModuleRunStepResult.ok("b", "ok");
                                    }
                                })
                        .withListeners(listener);
        ModuleRunReport report = pipeline.execute(ctx, carry);
        assertEquals(11, carry.value);
        assertTrue(report.allStepsSucceeded());
        assertEquals(2, report.stepResults().size());
        assertEquals(
                List.of("start", "step:a", "done:a:true", "step:b", "done:b:true", "finish"), events);
    }

    @Test
    void execute_propagatesModuleBackendException() {
        ModuleHostContext host = new ModuleHostContext(UUID.randomUUID(), UUID.randomUUID(), "m");
        ModuleRunContext ctx = ModuleRunContext.start(host);
        ModuleRunPipeline<Carry> pipeline =
                ModuleRunPipeline.of(
                        new ModuleRunStep<>() {
                            @Override
                            public String id() {
                                return "fail";
                            }

                            @Override
                            public ModuleRunStepResult run(ModuleRunContext c, Carry k) {
                                throw new ModuleBackendException("boom", "msg");
                            }
                        });
        assertThrows(ModuleBackendException.class, () -> pipeline.execute(ctx, new Carry()));
    }
}
