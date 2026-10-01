import { useEffect, useState } from "react";
import { listProviderModels } from "@/lib/desktop-api";
import { describeError } from "@/lib/utils";
import type { ProviderModel } from "@/types/domain";

/** Catalog reads start a provider process, so each provider is read once per app session. */
const catalogs = new Map<string, Promise<ProviderModel[]>>();

function loadCatalog(providerId: string): Promise<ProviderModel[]> {
  let catalog = catalogs.get(providerId);
  if (!catalog) {
    catalog = listProviderModels(providerId);
    catalogs.set(providerId, catalog);
    // A failed read is retried the next time a picker opens.
    catalog.catch(() => catalogs.delete(providerId));
  }
  return catalog;
}

/** Test-only: forgets cached catalogs between cases. */
export function resetProviderModelCache(): void {
  catalogs.clear();
}

export interface ProviderModels {
  models: ProviderModel[];
  loading: boolean;
  error?: string;
}

export function useProviderModels(providerId: string | undefined): ProviderModels {
  const [state, setState] = useState<ProviderModels>({ models: [], loading: false });

  useEffect(() => {
    if (!providerId) {
      setState({ models: [], loading: false });
      return;
    }
    let active = true;
    setState({ models: [], loading: true });
    loadCatalog(providerId).then(
      (models) => active && setState({ models, loading: false }),
      (caught: unknown) =>
        active &&
        setState({
          models: [],
          loading: false,
          error: describeError(caught, "Models could not be loaded.")
        })
    );
    return () => {
      active = false;
    };
  }, [providerId]);

  return state;
}
