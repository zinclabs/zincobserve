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
  <q-card class="column full-height">
    <q-card-section class="q-px-md q-py-md text-black">
      <div class="row items-center no-wrap">
        <div class="col">
          <div v-if="beingUpdated" class="text-h6">
            {{ t("user.editUser") }}
          </div>
          <div v-else class="text-h6">{{ t("user.add") }}</div>
        </div>
        <div class="col-auto">
          <q-btn
            v-close-popup
            round
            flat
            icon="img:/src/assets/images/common/close_icon.svg"
          />
        </div>
      </div>

      <q-separator />
      <div>
        <q-form ref="updateUserForm" @submit.prevent="onSubmit">
          <q-input
            v-if="!beingUpdated"
            v-model="formData.email"
            :label="t('user.email') + ' *'"
            color="input-border"
            bg-color="input-bg"
            class="q-py-md showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            :rules="[
              (val, rules) =>
                rules.email(val) || 'Please enter a valid email address',
            ]"
          />

          <div v-if="!beingUpdated">
            <q-input
              type="password"
              v-model="formData.password"
              :label="t('user.password') + ' *'"
              color="input-border"
              bg-color="input-bg"
              class="q-py-md showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              :rules="[
                (val) => !!val || 'Field is required',
                (val) =>
                  (val && val.length >= 8) ||
                  'Password must be at least 8 characters long',
                (val) =>
                  /^(?=.*[A-Z])(?=.*[a-z])(?=.*\d)(?=.*[@$!%*?&])[A-Za-z\d@$!%*?&]+$/.test(
                    val
                  ) ||
                  'Password must contain at least one uppercase letter, one lowercase letter, one number, and one special character',
              ]"
            />
          </div>

          <q-input
            v-model="formData.first_name"
            :label="t('user.firstName')"
            color="input-border"
            bg-color="input-bg"
            class="q-py-md showLabelOnTop q-mt-sm"
            stack-label
            outlined
            filled
            dense
          />

          <q-input
            v-model="formData.last_name"
            :label="t('user.lastName')"
            color="input-border"
            bg-color="input-bg"
            class="q-py-md showLabelOnTop"
            stack-label
            outlined
            filled
            dense
          />

          <q-select
            v-model="formData.role"
            :label="t('user.role') + ' *'"
            :options="roleOptions"
            color="input-border"
            bg-color="input-bg"
            class="q-pt-md q-pb-sm showLabelOnTop"
            stack-label
            outlined
            filled
            dense
          />

          <div v-if="beingUpdated">
            <q-toggle
              v-model="formData.change_password"
              :label="t('user.changePassword')"
              color="input-border"
              bg-color="input-bg"
              class="q-pt-md q-pb-sm showLabelOnTop"
              stack-label
              outlined
              filled
              dense
            />

            <q-input
              v-if="formData.change_password"
              type="password"
              v-model="formData.password"
              :label="t('user.password')"
              color="input-border"
              bg-color="input-bg"
              class="q-py-md showLabelOnTop"
              stack-label
              outlined
              filled
              dense
            />
          </div>

          <div class="flex justify-center q-mt-lg">
            <q-btn
              v-close-popup
              class="q-mb-md text-bold no-border"
              :label="t('user.cancel')"
              text-color="light-text"
              padding="sm md"
              color="accent"
              no-caps
            />
            <q-btn
              :label="t('user.save')"
              class="q-mb-md text-bold no-border q-ml-md"
              color="secondary"
              padding="sm xl"
              type="submit"
              no-caps
            />
          </div>
        </q-form>
      </div>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useQuasar } from "quasar";
import userServiece from "../../services/users";

const defaultValue: any = () => {
  return {
    org_member_id: "",
    role: "admin",
    first_name: "",
    last_name: "",
    email: "",
    password: "",
    change_password: false,
  };
};

export default defineComponent({
  name: "ComponentAddUpdateUser",
  props: {
    modelValue: {
      type: Object,
      default: () => defaultValue(),
    },
    isUpdated: {
      type: Boolean,
      default: false,
    },
  },
  emits: ["update:modelValue", "updated"],
  setup() {
    const store: any = useStore();
    const { t } = useI18n();
    const $q = useQuasar();
    const formData: any = ref(defaultValue());
    const roleOptions = ["admin", "user"];
    const beingUpdated: any = ref(false);
    const userForm: any = ref(null);

    return {
      t,
      $q,
      store,
      formData,
      beingUpdated,
      userForm,
      roleOptions,
    };
  },
  created() {
    this.formData = { ...defaultValue, ...this.modelValue };
    this.beingUpdated = this.isUpdated;

    if (
      this.modelValue &&
      this.modelValue.email != undefined &&
      this.modelValue.email != ""
    ) {
      this.beingUpdated = true;
      this.formData = this.modelValue;
      this.formData.change_password = false;
      this.formData.password = "";
    }
  },
  methods: {
    onSubmit() {
      const dismiss = this.$q.notify({
        spinner: true,
        message: "Please wait...",
        timeout: 2000,
      });

      this.formData.name =
        this.formData.first_name.trim() + this.formData.last_name.trim() != ""
          ? this.formData.first_name + " " + this.formData.last_name
          : "";

      userServiece
        .create(this.formData, this.store.state.selectedOrganization.identifier)
        .then((res: any) => {
          dismiss();
          this.$emit("updated", res.data);
        });
    },
  },
});
</script>
