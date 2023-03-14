<!-- Copyright 2022 Zinc Labs Inc. and Contributors

 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at

     http:www.apache.org/licenses/LICENSE-2.0

 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License. 
-->

<template>
  <q-card class="column full-height no-wrap" v-if="indexData.schema">
    <q-card-section class="q-pa-md">
      <div class="row items-center no-wrap">
        <div class="col">
          <div class="text-body1 text-bold text-dark">
            {{ t("logStream.schemaHeader") }}
          </div>
        </div>
        <div class="col-auto">
          <q-btn
            v-close-popup
            round
            flat
            :icon="'img:' + getImageURL('images/common/close_icon.svg')"
          />
        </div>
      </div>
    </q-card-section>
    <q-separator />
    <q-card-section>
      <q-form ref="updateSettingsForm" @submit.prevent="onSubmit">
        <div
          v-if="indexData.schema.length == 0"
          class="q-pt-md text-center q-w-md q-mx-lg"
          style="max-width: 450px"
        >
          No data available.
        </div>
        <div v-else class="indexDetailsContainer">
          <div class="title">{{ indexData.name }}</div>
          <div class="q-table__container q-table--cell-separator">
            <table class="q-table">
              <thead>
                <tr>
                  <th>{{ t("logStream.docsCount") }}</th>
                  <th>{{ t("logStream.storageSize") }}</th>
                  <th>{{ t("logStream.compressedSize") }}</th>
                  <th>{{ t("logStream.time") }}</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>
                    {{
                      parseInt(indexData.stats.doc_num).toLocaleString("en-US")
                    }}
                  </td>
                  <td>
                    {{
                      parseInt(indexData.stats.storage_size).toLocaleString(
                        "en-US"
                      )
                    }}
                    MB
                  </td>
                  <td>
                    {{
                      parseInt(indexData.stats.compressed_size).toLocaleString(
                        "en-US"
                      )
                    }}
                    MB
                  </td>
                  <td class="text-center">
                    {{ indexData.stats.doc_time_min }}
                    <br />
                    to
                    <br />
                    {{ indexData.stats.doc_time_max }}
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <q-separator class="q-mt-xl q-mb-lg" />

          <div class="title">
            {{ t("logStream.mapping") }}
            <label
              v-show="indexData.defaultFts"
              class="warning-msg"
              style="font-weight: normal"
              >- Using default fts keys, as no fts keys are set for
              stream.</label
            >
          </div>

          <!-- Note: Drawer max-height to be dynamically calculated with JS -->
          <div
            class="q-table__container q-table--cell-separator"
            style="height: calc(100vh - 460px); overflow: auto"
          >
            <table class="q-table">
              <thead>
                <tr>
                  <th>{{ t("logStream.propertyName") }}</th>
                  <th>{{ t("logStream.propertyType") }}</th>
                  <th>{{ t("logStream.streamftsKey") }}</th>
                  <th>{{ t("logStream.streamPartitionKey") }}</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="(schema, index) in indexData.schema"
                  :key="index + '_' + schema.name"
                  class="list-item"
                >
                  <td>{{ schema.name }}</td>
                  <td>{{ schema.type }}</td>
                  <td class="text-center">
                    <q-checkbox v-model="schema.ftsKey" size="sm" />
                  </td>
                  <td class="text-center">
                    <q-checkbox v-model="schema.partitionKey" size="sm">
                      {{ schema.level }}</q-checkbox
                    >
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <div
          v-if="indexData.schema.length > 0"
          class="flex justify-center q-mt-sm"
        >
          <q-btn
            v-close-popup
            class="q-mb-md text-bold no-border"
            :label="t('logStream.cancel')"
            text-color="light-text"
            padding="sm md"
            color="accent"
            no-caps
          />
          <q-btn
            :label="t('logStream.updateSettings')"
            class="q-mb-md text-bold no-border q-ml-md"
            color="secondary"
            padding="sm xl"
            type="submit"
            no-caps
          />
        </div>
      </q-form>
    </q-card-section>
  </q-card>
  <q-card v-else class="column q-pa-md full-height no-wrap">
    <h5>Wait while loading...</h5>
  </q-card>
</template>

<script lang="ts">
// @ts-nocheck
import { defineComponent, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useQuasar, date, format } from "quasar";
import streamService from "../../services/stream";
import segment from "../../services/segment_analytics";
import { getImageURL } from "../../utils/zincutils";

const defaultValue: any = () => {
  return {
    name: "",
    schema: [],
    stats: {},
    defaultFts: false,
  };
};

export default defineComponent({
  name: "SchemaIndex",
  props: {
    // eslint-disable-next-line vue/require-default-prop
    modelValue: {
      type: Object,
      default: () => defaultValue(),
    },
  },
  setup() {
    const { t } = useI18n();
    const store = useStore();
    const q = useQuasar();
    const indexData: any = ref(defaultValue());
    const updateSettingsForm: any = ref(null);

    const getSchema = async () => {
      const dismiss = q.notify({
        spinner: true,
        message: "Please wait while loading stats...",
      });

      await streamService
        .schema(
          store.state.selectedOrganization.identifier,
          indexData.value.name,
          indexData.value.stream_type
        )
        .then((res) => {
          res.data.stats.doc_time_max = date.formatDate(
            parseInt(res.data.stats.doc_time_max) / 1000,
            "YYYY-MM-DDTHH:mm:ss:SSZ"
          );
          res.data.stats.doc_time_min = date.formatDate(
            parseInt(res.data.stats.doc_time_min) / 1000,
            "YYYY-MM-DDTHH:mm:ss:SSZ"
          );

          if (res.data.settings.full_text_search_keys.length == 0) {
            indexData.value.defaultFts = true;
          } else {
            indexData.value.defaultFts = false;
          }

          indexData.value.schema = res.data.schema;
          indexData.value.stats = res.data.stats;

          for (var property of res.data.schema) {
            if (
              (res.data.settings.full_text_search_keys.length > 0 &&
                res.data.settings.full_text_search_keys.includes(
                  property.name
                )) ||
              (res.data.settings.full_text_search_keys.length == 0 &&
                store.state.zoConfig.default_fts_keys.includes(property.name))
            ) {
              property.ftsKey = true;
            } else {
              property.ftsKey = false;
            }

            if (
              res.data.settings.partition_keys &&
              Object.values(res.data.settings.partition_keys).includes(
                property.name
              )
            ) {
              let index = Object.values(
                res.data.settings.partition_keys
              ).indexOf(property.name);
              property.partitionKey = true;
              property.level = Object.keys(
                res.data.settings.partition_keys
              ).find(
                (key) => res.data.settings.partition_keys[key] === property.name
              );
            } else {
              property.partitionKey = false;
            }
          }

          dismiss();
        });
    };

    const onSubmit = async () => {
      /*  this.updateSettingsForm.validate().then((valid: any) => {
      if (!valid) {
          return false;
        } */

      let settings = { partition_keys: [], full_text_search_keys: [] };
      let added_part_keys = [];
      for (var property of indexData.value.schema) {
        if (property.ftsKey) {
          settings.full_text_search_keys.push(property.name);
        }
        if (property.level && property.partitionKey) {
          settings.partition_keys.push(property.name);
        } else if (property.partitionKey) {
          added_part_keys.push(property.name);
        }
      }
      if (added_part_keys.length > 0) {
        settings.partition_keys =
          settings.partition_keys.concat(added_part_keys);
      }

      await streamService
        .updateSettings(
          store.state.selectedOrganization.identifier,
          indexData.value.name,
          indexData.value.stream_type,
          settings
        )
        .then((res) => {
          getSchema();
          segment.track("Button Click", {
            button: "Update Settings",
            user_org: store.state.selectedOrganization.identifier,
            user_id: store.state.userInfo.email,
            stream_name: indexData.value.name,
            page: "Schema Details",
          });
          dismiss();
        });
    };

    return {
      t,
      q,
      indexData,
      getSchema,
      onSubmit,
      updateSettingsForm,
      format,
      getImageURL,
    };
  },
  created() {
    if (this.modelValue && this.modelValue.name) {
      this.indexData.name = this.modelValue.name;
      this.indexData.schema = this.modelValue.schema;
      this.indexData.stream_type = this.modelValue.stream_type;

      this.getSchema();
    }
  },
});
</script>

<style lang="scss">
.indexDetailsContainer {
  padding: 1.25rem;
  width: 100%;

  .title {
    margin-bottom: 1rem;
    font-weight: 700;
  }

  .q-table {
    border: 1px solid $input-field-border-color;
  }

  .q-table {
    border-radius: 0.5rem;

    thead tr {
      height: 2.5rem;

      th {
        font-size: 0.875rem;
        font-weight: 700;
        color: $dark;
      }
    }

    tbody tr {
      height: 3.25rem;

      td {
        font-size: 0.875rem;
        color: $dark-page;
        font-weight: 600;
      }
    }
  }

  .q-list {
    border-radius: 0 0 0.5rem 0.5rem;

    .q-item {
      height: 2.5rem;
      padding: 0;

      &__section {
        padding: 0.5rem 1rem;
        font-size: 0.875rem;
        color: $dark-page;

        &:not(:first-child) {
          border-left: 1px solid $input-field-border-color;
          align-items: flex-start;
          min-width: 29%;
        }
      }

      &.list-head {
        border: 1px solid $input-field-border-color;
        border-radius: 0.5rem 0.5rem 0 0;
        border-bottom: none;
      }

      &.list-item {
        border-right: 1px solid $input-field-border-color;
        border-left: 1px solid $input-field-border-color;

        &,
        &--side {
          font-weight: 600;
        }

        &:last-of-type {
          border-bottom: 1px solid $input-field-border-color;
          border-radius: 0 0 0.5rem 0.5rem;
        }
      }
    }
  }
}
</style>
