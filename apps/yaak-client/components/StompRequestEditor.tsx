import type { WebsocketRequest } from "@yaakapp-internal/models";
import { patchModelDebounced } from "@yaakapp-internal/models";
import { languageFromContentType } from "@yaakapp-internal/ui";
import { Button } from "./core/Button";
import { Editor } from "./core/Editor/LazyEditor";
import { PlainInput } from "./core/PlainInput";

interface Props {
  request: WebsocketRequest;
  fullHeight: boolean;
  forceUpdateKey: string;
  isConnected: boolean;
  onSubscribe: () => Promise<void>;
}

export function StompRequestEditor({
  request,
  fullHeight,
  forceUpdateKey,
  isConnected,
  onSubscribe,
}: Props) {
  const subscriptions = request.stompSubscriptions.join("\n");
  const messageLanguage = languageFromContentType(null, request.message);

  return (
    <div className="h-full min-h-0 grid grid-rows-[auto_auto_minmax(0,1fr)] gap-3 p-2">
      <div className="grid gap-1.5">
        <div className="flex items-center gap-2">
          <div className="text-sm font-medium">Subscriptions</div>
          <Button
            size="2xs"
            color="secondary"
            variant="border"
            disabled={!isConnected}
            title={isConnected ? "Subscribe to the listed topics" : "Connect first"}
            onClick={onSubscribe}
          >
            Subscribe
          </Button>
          <div className="text-xs text-text-subtlest">One destination per line</div>
        </div>
        <Editor
          forceUpdateKey={forceUpdateKey}
          autocompleteFunctions
          autocompleteVariables
          heightMode="auto"
          defaultValue={subscriptions}
          language="text"
          placeholder={"/topic/example\n/queue/example"}
          onChange={(value) =>
            patchModelDebounced(request, {
              stompSubscriptions: value
                .split(/\r?\n/)
                .map((destination) => destination.trim())
                .filter(Boolean),
            })
          }
          stateKey={`stomp-subscriptions.${request.id}`}
        />
      </div>
      <PlainInput
        label="Send Destination"
        defaultValue={request.stompDestination}
        placeholder="/topic/example"
        onChange={(stompDestination) => patchModelDebounced(request, { stompDestination })}
      />
      <div className="min-h-0 grid grid-rows-[auto_minmax(0,1fr)] gap-1.5">
        <div className="text-sm font-medium">Message</div>
        <Editor
          forceUpdateKey={forceUpdateKey}
          autocompleteFunctions
          autocompleteVariables
          placeholder="Message body"
          heightMode={fullHeight ? "full" : "auto"}
          defaultValue={request.message}
          language={messageLanguage}
          onChange={(message) => patchModelDebounced(request, { message })}
          stateKey={`stomp-message.${request.id}`}
        />
      </div>
    </div>
  );
}
